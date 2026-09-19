#!/usr/bin/env bash
# macOS memory-guard daemon (TCC-safe: runs only under $HOME/.local).
# Monitors free RAM, swap, compressor pressure, and heavy processes.
# Escalates automatically: notify -> emergency cull.
# Deterministic: the run loop never calls osascript. Month-old sessions
# notify + write a prompt file; `memory-guard review-sessions` is the only
# yes/no dialog. Classifier is a pure function covered by `selftest`.
set -u

VERSION="1.3.0"
LABEL="com.omnimon.memory-guard"

STATE_DIR="${MEMORY_GUARD_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/omnimon-memory-guard}"
LOG_FILE="${MEMORY_GUARD_LOG:-$STATE_DIR/memory-guard.log}"
STATUS_FILE="${MEMORY_GUARD_STATUS:-$STATE_DIR/status.env}"
METRICS_FILE="${MEMORY_GUARD_METRICS:-$STATE_DIR/metrics.csv}"
CONFIG_FILE="${MEMORY_GUARD_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/omnimon/memory-guard.conf}"
PID_FILE="${MEMORY_GUARD_PID:-$STATE_DIR/memory-guard.pid}"
LAST_ACTION_FILE="${MEMORY_GUARD_LAST_ACTION:-$STATE_DIR/last_action.txt}"
LAST_SESSION_PROMPT_FILE="${MEMORY_GUARD_LAST_SESSION_PROMPT:-$STATE_DIR/last_session_prompt.txt}"

# Defaults (overridable via config or env)
: "${MG_INTERVAL_OK:=15}"
: "${MG_INTERVAL_STRESS:=5}"
: "${MG_FREE_WARN:=25}"
: "${MG_FREE_CRIT:=15}"
: "${MG_FREE_EMERGENCY:=8}"
: "${MG_SWAP_WARN_MB:=2048}"
: "${MG_SWAP_CRIT_MB:=6144}"
: "${MG_SWAP_EMERGENCY_MB:=12288}"
: "${MG_COMPRESSOR_WARN_MB:=12288}"
: "${MG_COMPRESSOR_CRIT_MB:=20480}"
: "${MG_PROC_KILL_MB:=6144}"
: "${MG_CHROME_HELPER_MIN_MB:=400}"
: "${MG_CHROME_TOTAL_CRIT_MB:=12288}"
: "${MG_CHROME_TOTAL_EMERGENCY_MB:=20480}"
: "${MG_AUTO_KILL:=1}"
: "${MG_NOTIFY:=1}"
: "${MG_DRY_RUN:=0}"
: "${MG_LOG_EVERY_OK:=12}"
: "${MG_MAX_KILLS_PER_CYCLE:=6}"
: "${MG_COOLDOWN_SEC:=60}"
: "${MG_EMPTY_CULL_COOLDOWN_SEC:=120}"
: "${MG_NOTIFY_COOLDOWN_SEC:=180}"
: "${MG_NOTIFY_MIN_GAP_SEC:=60}"
: "${MG_REAP_MIN_AGE_SEC:=3600}"
: "${MG_REAP_ANCIENT_SEC:=86400}"
: "${MG_REAP_MAX_CPU:=0.5}"
: "${MG_REAP_SERVER_CHECK:=1}"
: "${MG_SESSION_AGE_SEC:=2592000}"
: "${MG_SESSION_SNOOZE_SEC:=604800}"
: "${MG_SESSION_DIALOG:=1}"
: "${MG_SESSION_DIALOG_TIMEOUT:=180}"
: "${MG_SESSION_PROMPT_GAP_SEC:=86400}"

LAST_ACTION_TS=0
LAST_NOTIFY_LEVEL=""
LAST_NOTIFY_SIG=""
OK_TICKS=0
SELF_PID=$$

# maybe_act runs in a command substitution (subshell). Persist cooldown on disk
# so crit notifications do not fire every sample and then vanish.
read_last_action() {
  LAST_ACTION_TS=0
  LAST_NOTIFY_LEVEL=""
  LAST_NOTIFY_SIG=""
  if [[ -f "$LAST_ACTION_FILE" ]]; then
    LAST_ACTION_TS="$(/usr/bin/sed -n '1p' "$LAST_ACTION_FILE" 2>/dev/null || true)"
    LAST_NOTIFY_LEVEL="$(/usr/bin/sed -n '2p' "$LAST_ACTION_FILE" 2>/dev/null || true)"
    LAST_NOTIFY_SIG="$(/usr/bin/sed -n '3p' "$LAST_ACTION_FILE" 2>/dev/null || true)"
  fi
  [[ "$LAST_ACTION_TS" =~ ^[0-9]+$ ]] || LAST_ACTION_TS=0
}

write_last_action() {
  mkdir -p "$(dirname "$LAST_ACTION_FILE")" 2>/dev/null || true
  printf '%s\n%s\n%s\n' "${LAST_ACTION_TS:-0}" "${LAST_NOTIFY_LEVEL:-}" "${LAST_NOTIFY_SIG:-}" >"$LAST_ACTION_FILE" 2>/dev/null || true
}

load_config() {
  if [[ -f "$CONFIG_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$CONFIG_FILE" || true
  fi
}

ensure_dirs() {
  mkdir -p "$STATE_DIR" "$(dirname "$CONFIG_FILE")" 2>/dev/null || true
  touch "$LOG_FILE" "$METRICS_FILE" 2>/dev/null || true
  if [[ ! -s "$METRICS_FILE" ]]; then
    printf 'ts,free_pct,swap_mb,compressor_mb,chrome_mb,top_rss_mb,level,action\n' >"$METRICS_FILE" 2>/dev/null || true
  fi
}

log() {
  local msg="$*"
  printf '%s %s\n' "$(date '+%Y-%m-%dT%H:%M:%S%z')" "$msg" >>"$LOG_FILE" 2>/dev/null || true
}

notify() {
  local body="$1"
  local title="${2:-Memory Guard}"
  [[ "${MG_NOTIFY}" == "1" ]] || return 0
  /usr/bin/osascript -e "display notification \"${body//\"/\\\"}\" with title \"${title//\"/\\\"}\"" >/dev/null 2>&1 || true
}

# --- metric parsers (testable) ------------------------------------------------

parse_free_pct() {
  # stdin: memory_pressure output
  /usr/bin/awk '
    /System-wide memory free percentage/ {
      gsub(/%/, "", $NF)
      print int($NF + 0)
      found = 1
      exit
    }
    END {
      if (!found) print -1
    }
  '
}

parse_swap_used_mb() {
  # stdin: sysctl vm.swapusage line — values like "used = 512.50M"
  /usr/bin/awk '
    {
      for (i = 1; i <= NF; i++) {
        if ($i == "used") {
          val = $(i + 2)
          gsub(/[^0-9.]/, "", val)
          if (val == "") val = 0
          printf "%d\n", int(val + 0.5)
          found = 1
          exit
        }
      }
    }
    END {
      if (!found) print 0
    }
  '
}

parse_compressor_mb() {
  # stdin: vm_stat output; page size passed as $1 (bytes)
  local page_size="${1:-16384}"
  /usr/bin/awk -v ps="$page_size" '
    /Pages occupied by compressor/ {
      gsub(/\./, "", $NF)
      pages = $NF + 0
      printf "%d\n", int((pages * ps) / 1024 / 1024 + 0.5)
      found = 1
      exit
    }
    END {
      if (!found) print 0
    }
  '
}

is_protected_name() {
  local name="$1"
  case "$name" in
    kernel_task|launchd|WindowServer|loginwindow|SystemUIServer|Dock|Finder|\
    cfprefsd|distnoted|notifyd|syslogd|UserEventAgent|coreservicesd|\
    mDNSResponder|coreaudiod|bluetoothd|securityd|trustd|airportd|\
    opendirectoryd|runningboardd|dasd|powerd|thermalmonitord|\
    memory-guard|memory-guard.sh|sshd|ssh|ScreenSaverEngine|\
    Warp|Terminal|iTerm2|iTerm|Cursor|ChatGPT|Code|\
    fseventsd|mds|mds_stores|corespotlightd)
      return 0
      ;;
  esac
  return 1
}

# Interactive host apps are never leftovers. Warp's on-disk binary is named
# `stable` and often runs as `stable --finish-update` while it is the live app
# with all the operator's sessions. That flag is not a stuck updater.
is_host_app() {
  local spec="${1:-}"
  case "$spec" in
    *"/Applications/Warp.app"*|*"/Warp.app/"*) return 0 ;;
    *"/Applications/Utilities/Terminal.app"*|*"/Terminal.app/"*) return 0 ;;
    *"/Applications/iTerm.app"*|*iTerm2*|*"/Alacritty.app"*|*"/kitty.app"*|*"/Ghostty.app"*) return 0 ;;
    *"/Applications/Cursor.app"*|*"Visual Studio Code.app"*|*"Code.app/Contents/MacOS/Code"*) return 0 ;;
    *"/Applications/ChatGPT.app"*|*"Codex Framework"*) return 0 ;;
    *"/Applications/Activity Monitor.app"*) return 0 ;;
    *"/Applications/Finder.app"*|*"/System/Library/CoreServices/Finder.app"*) return 0 ;;
    *"Runner.Listener"*|*"Runner.Worker"*|"/Users/"*"/actions-runner"*) return 0 ;;
  esac
  # Chrome main binary (helpers are not hosts)
  case "$spec" in
    *"Helper"*) ;;
    *"/MacOS/Google Chrome"*|*"/MacOS/Chromium"*) return 0 ;;
  esac
  local first base
  first="${spec%% *}"
  base="$(/usr/bin/basename "$first" 2>/dev/null || echo "$spec")"
  case "$base" in
    Warp|Terminal|iTerm2|iTerm|Cursor|ChatGPT|Code|Finder|Dock) return 0 ;;
    stable)
      case "$spec" in *Warp.app*|*"--finish-update"*) return 0 ;; esac
      ;;
    claude|opencode|grok) return 0 ;;
  esac
  case "$spec" in
    claude\ *|*"/bin/claude "*|opencode\ *|*"/.opencode/bin/opencode"*|grok|grok\ *) return 0 ;;
    *"firebase-tools"*mcp*|*"playwright/mcp"*|*"chrome-devtools-mcp"*|*"mcp-pdf"*|*"/bin/firebase mcp"*|*playwright-mcp*) return 0 ;;
    *"dart mcp-server"*|*"language-server"*|*"analysis_server"*) return 0 ;;
    *"Cursor Helper"*|*"mcp-process"*) return 0 ;;
  esac
  return 1
}

# The window the operator lives in. One dialog for Warp, not one per Claude.
is_session_container() {
  local spec="${1:-}"
  case "$spec" in
    *terminal-server*|*Helper*|*crashpad*) return 1 ;;
    *"/Applications/Warp.app/Contents/MacOS/stable"*) return 0 ;;
    *"/Applications/Cursor.app/Contents/MacOS/Cursor"*) return 0 ;;
    *"/Applications/ChatGPT.app/Contents/MacOS/ChatGPT"*) return 0 ;;
    *"/Applications/OpenCode.app/Contents/MacOS/"*) return 0 ;;
  esac
  return 1
}

# Agent CLIs. Only prompt if orphaned (ppid 1). Inside Warp they are the session.
is_agent_cli() {
  local spec="${1:-}"
  case "$spec" in
    *Helper*|*crashpad*|*"npm exec"*|*flutter_tester*) return 1 ;;
    claude\ --*|*"/.local/bin/claude "*|*"/.claude/local/claude "*) return 0 ;;
    *"/opencode --auto"*|*"/.opencode/bin/opencode"*) return 0 ;;
    grok\ --*|*"/.grok/"*"grok "*) return 0 ;;
  esac
  return 1
}

is_session_host() {
  is_session_container "$1" && return 0
  is_agent_cli "$1" && return 0
  return 1
}

session_days() {
  local sec
  sec="$(etime_seconds "${1:-0}")"
  printf '%s\n' $((sec / 86400))
}

# Warp/Cursor/ChatGPT always. Agent CLIs only if the parent died (ppid 1).
session_should_prompt() {
  local ppid="${1:-}" cmd="${2:-}"
  if is_session_container "$cmd"; then
    return 0
  fi
  if is_agent_cli "$cmd" && [[ "$ppid" == "1" ]]; then
    return 0
  fi
  return 1
}

session_prompt_recent() {
  local now last
  now="$(date +%s)"
  last="$(cat "$LAST_SESSION_PROMPT_FILE" 2>/dev/null || echo 0)"
  [[ "$last" =~ ^[0-9]+$ ]] || last=0
  (( now - last < MG_SESSION_PROMPT_GAP_SEC ))
}

mark_session_prompt() {
  mkdir -p "$(dirname "$LAST_SESSION_PROMPT_FILE")"
  date +%s >"$LAST_SESSION_PROMPT_FILE"
}

etime_seconds() {
  local s="${1:-0}" days=0 a=0 b=0 c=""
  if [[ "$s" == *-* ]]; then
    days="${s%%-*}"
    s="${s#*-}"
  fi
  IFS=: read -r a b c <<<"$s"
  days=$((10#${days:-0}))
  a=$((10#${a:-0}))
  b=$((10#${b:-0}))
  # macOS etime is mm:ss under 1h, hh:mm:ss under 1d, dd-hh:mm:ss after that.
  if [[ -n "${c}" ]]; then
    c=$((10#${c}))
    printf '%s\n' $((days * 86400 + a * 3600 + b * 60 + c))
  else
    printf '%s\n' $((days * 86400 + a * 60 + b))
  fi
}

humanize_proc() {
  local cmd="${1:-}"
  case "$cmd" in
    *"Chrome Helper (Renderer)"*|*"Chrome Helper"*) printf '%s' "Chrome (pestaña)" ;;
    *"/MacOS/Google Chrome"*|"Google Chrome") printf '%s' "Chrome" ;;
    *Warp.app*|*"--finish-update"*) printf '%s' "Warp" ;;
    *flutter_tester*) printf '%s' "flutter_tester" ;;
    *language-server*) printf '%s' "dart (language-server)" ;;
    *dartaotruntime*|*"/bin/dart "*|*"/bin/dart") printf '%s' "dart" ;;
    claude\ *|*"/bin/claude "*) printf '%s' "Claude" ;;
    *opencode*) printf '%s' "OpenCode" ;;
    grok|grok\ *) printf '%s' "Grok" ;;
    *Cursor.app*) printf '%s' "Cursor" ;;
    *ChatGPT.app*) printf '%s' "ChatGPT" ;;
    *"-m http.server"*) printf '%s' "http.server" ;;
    *fseventsd*) printf '%s' "fseventsd" ;;
    *"fsmonitor--daemon"*) printf '%s' "git fsmonitor" ;;
    *"firebase-tools"*) printf '%s' "Firebase MCP" ;;
    *"playwright/mcp"*|*playwright-mcp*) printf '%s' "Playwright MCP" ;;
    *"chrome-devtools-mcp"*) printf '%s' "Chrome DevTools MCP" ;;
    *"mcp-server"*) printf '%s' "dart MCP" ;;
    *php*" -S "*) printf '%s' "php -S" ;;
    next-server*) printf '%s' "next-server" ;;
    *Runner.Listener*) printf '%s' "GitHub runner" ;;
    *)
      local first
      first="${cmd%% *}"
      /usr/bin/basename "$first" 2>/dev/null || printf '%s' "$first"
      ;;
  esac
}

format_size_mb() {
  local mb="${1:-0}"
  if (( mb >= 1024 )); then
    /usr/bin/awk -v m="$mb" 'BEGIN { printf "%.1f GB", m / 1024 }'
  else
    printf '%s MB' "$mb"
  fi
}

# stdin: rss_mb|pid|command  -> "Chrome (pestaña) está usando 1.2 GB · dart 775 MB"
format_offenders() {
  local max="${1:-3}" line mb rest cmd label size n=0 out=""
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    (( n >= max )) && break
    mb="${line%%|*}"
    rest="${line#*|}"
    cmd="${rest#*|}"
    label="$(humanize_proc "$cmd")"
    size="$(format_size_mb "$mb")"
    if (( n == 0 )); then
      out="${label} está usando ${size}"
    else
      out="${out} · ${label} ${size}"
    fi
    n=$((n + 1))
  done
  printf '%s' "$out"
}

leftover_kind() {
  local cmd="${1:-}" base
  case "$cmd" in
    *flutter_tester*) printf '%s\n' "flutter_tester"; return 0 ;;
    *"-m http.server"*) printf '%s\n' "http.server"; return 0 ;;
    next-server*) printf '%s\n' "next-server"; return 0 ;;
  esac
  case "$cmd" in
    *php*)
      case "$cmd" in
        *" -S "*) printf '%s\n' "php"; return 0 ;;
      esac
      ;;
  esac
  case "$cmd" in
    *"fsmonitor--daemon"*) printf '%s\n' "git-fsmonitor"; return 0 ;;
  esac
  base="$(/usr/bin/basename "${cmd%% *}" 2>/dev/null || echo "")"
  case "$base" in
    head|tail|ugrep|cat) printf '%s\n' "orphan-pipe"; return 0 ;;
  esac
  printf '%s\n' ""
  return 1
}

is_zombie_state() {
  local state="${1:-}" cmd="${2:-}"
  case "$state" in
    *Z*|*z*) return 0 ;;
  esac
  case "$cmd" in
    *"<defunct>"*|*"defunct"*) return 0 ;;
  esac
  return 1
}

cpu_is_idle() {
  local cpu="${1:-}"
  [[ -z "$cpu" ]] && return 0
  /usr/bin/awk -v c="$cpu" -v max="${MG_REAP_MAX_CPU:-0.5}" 'BEGIN { exit (c+0 <= max+0) ? 0 : 1 }'
}

is_server_kind() {
  case "${1:-}" in
    http.server|php|next-server) return 0 ;;
  esac
  return 1
}

server_has_clients() {
  local pid="${1:-}"
  [[ "${MG_REAP_SERVER_CHECK:-1}" == "1" ]] || return 1
  [[ -n "$pid" ]] || return 1
  # lsof can hang; 2s cap so reap is always bounded.
  /usr/bin/perl -e 'alarm 2; exec @ARGV' /usr/sbin/lsof -nP -p "$pid" -iTCP 2>/dev/null | /usr/bin/grep -q ESTABLISHED
}

# Unused leftovers only. A process with a living parent is in use
# (flutter_tester under flutter test, npm MCP under Claude/Cursor/OpenCode).
# args: ppid etime cmd [state] [cpu]
is_reapable_leftover() {
  local ppid="${1:-}" etime="${2:-0}" cmd="${3:-}" state="${4:-}" cpu="${5:-}"
  local sec kind
  if is_host_app "$cmd"; then
    return 1
  fi
  local first base
  first="${cmd%% *}"
  base="$(/usr/bin/basename "$first" 2>/dev/null || echo "$cmd")"
  if is_protected_name "$base"; then
    return 1
  fi
  kind="$(leftover_kind "$cmd" || true)"
  if is_zombie_state "$state" "$cmd"; then
    [[ -n "$kind" || "$ppid" == "1" ]] && return 0
    return 1
  fi
  [[ -n "$kind" ]] || return 1
  # Living parent = still attached to a test, agent, or IDE. Never reap.
  [[ "$ppid" == "1" ]] || return 1
  cpu_is_idle "$cpu" || return 1
  sec="$(etime_seconds "$etime")"
  if (( sec >= MG_REAP_MIN_AGE_SEC )); then
    return 0
  fi
  return 1
}

is_chrome_family() {
  local name="$1"
  case "$name" in
    *Chrome*|*Chromium*|*Google\ Chrome*) return 0 ;;
  esac
  return 1
}

# Main browser binary only (not Helper / Renderer / GPU / Plugin).
is_chrome_main() {
  local name="$1" base
  case "$name" in
    *"Helper"*) return 1 ;;
  esac
  case "$name" in
    "Google Chrome"|"Chromium"|"Google Chrome Canary"|"Google Chrome Beta"|"Google Chrome Dev")
      return 0
      ;;
    *"/MacOS/Google Chrome"*|*"/MacOS/Chromium"*)
      return 0
      ;;
  esac
  base="$(/usr/bin/basename "$name" 2>/dev/null || echo "$name")"
  case "$base" in
    "Google Chrome"|"Chromium"|"Google Chrome Canary"|"Google Chrome Beta"|"Google Chrome Dev")
      return 0
      ;;
  esac
  return 1
}

is_chrome_helper() {
  local name="$1" base
  base="$(/usr/bin/basename "$name" 2>/dev/null || echo "$name")"
  if is_chrome_main "$name" || is_chrome_main "$base"; then
    return 1
  fi
  if is_chrome_family "$name" || is_chrome_family "$base"; then
    case "$base" in
      *Helper*|*Renderer*|*GPU*|*Plugin*|*crashpad*) return 0 ;;
    esac
    # other chrome-family non-main processes count as helpers
    return 0
  fi
  return 1
}

level_from_metrics() {
  # args: free_pct swap_mb chrome_mb top_rss_mb [compressor_mb]
  local free_pct="$1" swap_mb="$2" chrome_mb="$3" top_rss_mb="$4" compressor_mb="${5:-0}"
  local level="ok"

  if (( free_pct >= 0 && free_pct < MG_FREE_WARN )) || (( swap_mb >= MG_SWAP_WARN_MB )) \
    || (( compressor_mb >= MG_COMPRESSOR_WARN_MB )); then
    level="warn"
  fi
  if (( free_pct >= 0 && free_pct < MG_FREE_CRIT )) || (( swap_mb >= MG_SWAP_CRIT_MB )) \
    || (( chrome_mb >= MG_CHROME_TOTAL_CRIT_MB )) || (( top_rss_mb >= MG_PROC_KILL_MB )) \
    || (( compressor_mb >= MG_COMPRESSOR_CRIT_MB )); then
    level="crit"
  fi
  # Emergency = real thrash (free collapsed or Chrome balloon), NOT swap hangover alone.
  # High swap after a spike with free still OK must stay at crit/warn so we do not
  # SIGKILL the main Chrome process.
  if (( free_pct >= 0 && free_pct < MG_FREE_EMERGENCY )) \
    || (( chrome_mb >= MG_CHROME_TOTAL_EMERGENCY_MB )); then
    level="emergency"
  elif (( swap_mb >= MG_SWAP_EMERGENCY_MB )) && (( free_pct >= 0 && free_pct < MG_FREE_CRIT )); then
    level="emergency"
  fi
  printf '%s\n' "$level"
}

# Fixed cadence. A 5s crit loop made launchd mark the agent "inefficient"
# and delay it, so sampling looked random. Notify rate is the cooldown file.
interval_for_level() {
  printf '%s\n' "${MG_INTERVAL_OK}"
}

# --- live collectors ----------------------------------------------------------

page_size_bytes() {
  /usr/bin/pagesize 2>/dev/null || echo 16384
}

collect_free_pct() {
  /usr/bin/memory_pressure 2>/dev/null | parse_free_pct
}

collect_swap_mb() {
  /usr/sbin/sysctl -n vm.swapusage 2>/dev/null | parse_swap_used_mb
}

collect_compressor_mb() {
  local ps
  ps="$(page_size_bytes)"
  /usr/bin/vm_stat 2>/dev/null | parse_compressor_mb "$ps"
}

# Prints lines: rss_mb|pid|command  (full command so Warp.app / --finish-update is visible)
list_heavy_processes() {
  # rss is KB in ps
  /bin/ps -axo pid=,rss=,command= 2>/dev/null | /usr/bin/awk '
    NF >= 3 {
      pid = $1
      rss_kb = $2 + 0
      $1 = ""
      $2 = ""
      sub(/^  */, "", $0)
      cmd = $0
      mb = int(rss_kb / 1024)
      if (mb >= 64) printf "%d|%s|%s\n", mb, pid, cmd
    }
  ' | /usr/bin/sort -t'|' -k1,1nr
}

sum_chrome_mb() {
  local total=0 line mb
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    mb="${line%%|*}"
    rest="${line#*|}"
    pid="${rest%%|*}"
    name="${rest#*|}"
    base="$(/usr/bin/basename "$name" 2>/dev/null || echo "$name")"
    if is_chrome_family "$name" || is_chrome_family "$base"; then
      total=$((total + mb))
    fi
  done
  printf '%d\n' "$total"
}

top_rss_mb_from_list() {
  local line="$1"
  if [[ -z "$line" ]]; then
    echo 0
    return
  fi
  printf '%s\n' "${line%%|*}"
}

daemon_pid_for_status() {
  local pid=""
  if [[ -f "$PID_FILE" ]]; then
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "${pid:-}" ]] && /bin/kill -0 "$pid" 2>/dev/null; then
      printf '%s\n' "$pid"
      return 0
    fi
  fi
  printf '%s\n' "$SELF_PID"
}

write_status() {
  local free_pct="$1" swap_mb="$2" compressor_mb="$3" chrome_mb="$4" top_rss_mb="$5" level="$6" action="$7"
  local dpid
  dpid="$(daemon_pid_for_status)"
  cat >"$STATUS_FILE" <<EOF
ts=$(date +%s)
iso=$(date -u +%Y-%m-%dT%H:%M:%SZ)
free_pct=$free_pct
swap_mb=$swap_mb
compressor_mb=$compressor_mb
chrome_mb=$chrome_mb
top_rss_mb=$top_rss_mb
level=$level
last_action=$action
version=$VERSION
pid=$dpid
EOF
  printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "$free_pct" "$swap_mb" "$compressor_mb" "$chrome_mb" "$top_rss_mb" "$level" "$action" \
    >>"$METRICS_FILE" 2>/dev/null || true
}

safe_kill_pid() {
  local pid="$1" reason="$2"
  local name cmd base
  name="$(/bin/ps -p "$pid" -o comm= 2>/dev/null || echo unknown)"
  cmd="$(/bin/ps -p "$pid" -o command= 2>/dev/null || echo "$name")"
  base="$(/usr/bin/basename "${name%% *}" 2>/dev/null || echo "$name")"

  if [[ "$pid" == "$SELF_PID" ]]; then
    return 1
  fi
  if is_host_app "$cmd" || is_host_app "$name"; then
    log "skip host pid=$pid name=$name cmd=$cmd"
    return 1
  fi
  if is_protected_name "$base" || is_protected_name "$name"; then
    log "skip protected pid=$pid name=$name"
    return 1
  fi
  # Never kill PID 0/1
  if (( pid <= 1 )); then
    return 1
  fi

  if [[ "$MG_DRY_RUN" == "1" ]]; then
    log "DRY_RUN would kill pid=$pid name=$name reason=$reason"
    return 0
  fi

  log "SIGTERM pid=$pid name=$name reason=$reason"
  /bin/kill -TERM "$pid" 2>/dev/null || true
  /bin/sleep 2
  if /bin/kill -0 "$pid" 2>/dev/null; then
    log "SIGKILL pid=$pid name=$name reason=$reason"
    /bin/kill -KILL "$pid" 2>/dev/null || true
  fi
  return 0
}

# Emergency only: kill largest Chrome helpers first, then other huge non-protected procs.
# Never kill the main Google Chrome binary unless its own RSS >= MG_PROC_KILL_MB.
cull_memory() {
  local mode="$1" # crit|emergency
  local kills=0
  local line mb pid name base rest
  local list
  list="$(list_heavy_processes)"

  # Pass 1: Chrome helpers / renderers only (never main browser here)
  while IFS= read -r line; do
    (( kills >= MG_MAX_KILLS_PER_CYCLE )) && break
    [[ -z "$line" ]] && continue
    mb="${line%%|*}"
    rest="${line#*|}"
    pid="${rest%%|*}"
    name="${rest#*|}"
    base="$(/usr/bin/basename "$name" 2>/dev/null || echo "$name")"

    if is_chrome_main "$name" || is_chrome_main "$base"; then
      continue
    fi
    if ! is_chrome_helper "$name" && ! is_chrome_helper "$base"; then
      continue
    fi
    if (( mb < MG_CHROME_HELPER_MIN_MB )); then
      continue
    fi
    if safe_kill_pid "$pid" "chrome-helper-$mode-${mb}MB"; then
      kills=$((kills + 1))
    fi
  done <<<"$list"

  # Pass 2: any process over hard RSS threshold (includes main Chrome only if huge)
  if [[ "$mode" == "emergency" ]] || (( kills == 0 )); then
    while IFS= read -r line; do
      (( kills >= MG_MAX_KILLS_PER_CYCLE )) && break
      [[ -z "$line" ]] && continue
      mb="${line%%|*}"
      rest="${line#*|}"
      pid="${rest%%|*}"
      name="${rest#*|}"
      base="$(/usr/bin/basename "$name" 2>/dev/null || echo "$name")"
      if (( mb < MG_PROC_KILL_MB )); then
        continue
      fi
      if is_host_app "$name" || is_protected_name "$base" || is_protected_name "$name"; then
        continue
      fi
      # Main Chrome only when that single process is truly enormous
      if is_chrome_main "$name" || is_chrome_main "$base"; then
        if (( mb < MG_PROC_KILL_MB )); then
          continue
        fi
      fi
      if safe_kill_pid "$pid" "rss-$mode-${mb}MB"; then
        kills=$((kills + 1))
      fi
    done <<<"$list"
  fi

  printf '%d\n' "$kills"
}

maybe_act() {
  local level="$1" free_pct="$2" swap_mb="$3" chrome_mb="$4" top_rss_mb="$5" offenders="${6:-}"
  local now action="none" kills=0 cooldown="$MG_COOLDOWN_SEC"
  local sig gap
  now="$(date +%s)"
  read_last_action
  sig="${level}|${offenders}"

  # Even emergency respects cooldown after an empty cull (swap hangover, nothing safe to kill).
  if [[ "$level" == "emergency" ]] && (( now - LAST_ACTION_TS < cooldown )); then
    printf '%s\n' "cooldown"
    return 0
  fi

  case "$level" in
    ok)
      action="none"
      ;;
    warn|crit)
      gap="$MG_NOTIFY_COOLDOWN_SEC"
      if [[ "$sig" != "$LAST_NOTIFY_SIG" ]]; then
        gap="$MG_NOTIFY_MIN_GAP_SEC"
      fi
      if (( now - LAST_ACTION_TS < gap )); then
        printf '%s\n' "cooldown"
        return 0
      fi
      action="notify"
      if [[ "$level" == "crit" ]]; then
        if [[ -n "$offenders" ]]; then
          case "$offenders" in
            *fseventsd*)
              notify "Crítico: ${offenders}. Es de Apple (cambios de archivos). No lo cierro. Para soltar RAM: memory-guard relieve-fsevents" "Memory Guard · crit"
              ;;
            *)
              notify "Crítico: libre ${free_pct}% · swap ${swap_mb} MB · ${offenders}. Sin cierres de apps." "Memory Guard · crit"
              ;;
          esac
        else
          notify "Crítico: libre ${free_pct}% · swap ${swap_mb} MB · Chrome ${chrome_mb} MB. Sin cierres de apps." "Memory Guard · crit"
        fi
        log "CRIT free=${free_pct}% swap=${swap_mb}MB chrome=${chrome_mb}MB top=${top_rss_mb}MB offenders=${offenders}"
      else
        if [[ -n "$offenders" ]]; then
          notify "RAM libre ${free_pct}% · swap ${swap_mb} MB · ${offenders}" "Memory Guard · warn"
        else
          notify "RAM libre ${free_pct}% · swap ${swap_mb}MB · Chrome ${chrome_mb}MB" "Memory Guard · warn"
        fi
        log "WARN free=${free_pct}% swap=${swap_mb}MB chrome=${chrome_mb}MB top=${top_rss_mb}MB offenders=${offenders}"
      fi
      LAST_ACTION_TS=$now
      LAST_NOTIFY_LEVEL="$level"
      LAST_NOTIFY_SIG="$sig"
      write_last_action
      ;;
    emergency)
      action="notify"
      if [[ -n "$offenders" ]]; then
        notify "EMERGENCIA: ${offenders}. Cierro helpers/pestañas, no Warp ni el browser entero." "Memory Guard · emergency"
      else
        notify "EMERGENCIA memoria: cerrando tabs/helpers pesados (no el browser entero salvo RSS enorme)" "Memory Guard · emergency"
      fi
      log "EMERGENCY free=${free_pct}% swap=${swap_mb}MB chrome=${chrome_mb}MB top=${top_rss_mb}MB offenders=${offenders}"
      if [[ "$MG_AUTO_KILL" == "1" ]]; then
        kills="$(cull_memory emergency)"
        action="cull_emergency:${kills}"
        log "EMERGENCY cull killed=$kills"
        if (( kills == 0 )); then
          # Avoid thrashing the daemon every 5s when only swap hangover remains.
          LAST_ACTION_TS=$((now + MG_EMPTY_CULL_COOLDOWN_SEC - cooldown))
        else
          LAST_ACTION_TS=$now
        fi
      else
        LAST_ACTION_TS=$now
      fi
      LAST_NOTIFY_LEVEL="$level"
      LAST_NOTIFY_SIG="$sig"
      write_last_action
      ;;
  esac

  printf '%s\n' "$action"
}

sample_once() {
  local free_pct swap_mb compressor_mb list chrome_mb top_line top_rss_mb level action offenders
  free_pct="$(collect_free_pct)"
  swap_mb="$(collect_swap_mb)"
  compressor_mb="$(collect_compressor_mb)"
  list="$(list_heavy_processes)"
  chrome_mb="$(printf '%s\n' "$list" | sum_chrome_mb)"
  top_line="$(printf '%s\n' "$list" | /usr/bin/head -n 1)"
  top_rss_mb="$(top_rss_mb_from_list "$top_line")"
  offenders="$(printf '%s\n' "$list" | format_offenders 3)"
  level="$(level_from_metrics "$free_pct" "$swap_mb" "$chrome_mb" "$top_rss_mb" "$compressor_mb")"
  action="$(maybe_act "$level" "$free_pct" "$swap_mb" "$chrome_mb" "$top_rss_mb" "$offenders")"
  write_status "$free_pct" "$swap_mb" "$compressor_mb" "$chrome_mb" "$top_rss_mb" "$level" "$action"

  if [[ "$level" != "ok" ]] || (( OK_TICKS % MG_LOG_EVERY_OK == 0 )); then
    log "sample level=$level free=${free_pct}% swap=${swap_mb}MB compressor=${compressor_mb}MB chrome=${chrome_mb}MB top=${top_rss_mb}MB action=$action"
  fi
  if [[ "$level" == "ok" ]]; then
    OK_TICKS=$((OK_TICKS + 1))
  else
    OK_TICKS=0
  fi

  printf '%s\n' "$level"
}

classify_proc() {
  local cmd="${1:-}"
  if is_host_app "$cmd"; then
    printf '%s\n' "host"
    return
  fi
  if is_chrome_helper "$cmd"; then
    printf '%s\n' "chrome-helper"
    return
  fi
  printf '%s\n' "other"
}

cmd_offenders() {
  local line mb rest pid cmd label class
  load_config
  printf '%-14s %8s %8s  %s\n' "CLASS" "RSS" "PID" "NAME"
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    mb="${line%%|*}"
    rest="${line#*|}"
    pid="${rest%%|*}"
    cmd="${rest#*|}"
    label="$(humanize_proc "$cmd")"
    class="$(classify_proc "$cmd")"
    printf '%-14s %6sMB %8s  %s\n' "$class" "$mb" "$pid" "$label"
  done < <(list_heavy_processes | /usr/bin/head -n 20)
}

cmd_reap() {
  local pid ppid etime cpu state cmd kills=0 skipped=0 line kind pcmd
  load_config
  ensure_dirs
  while IFS=$'\t' read -r pid ppid etime cpu state cmd; do
    [[ -z "${pid:-}" ]] && continue
    if is_host_app "$cmd"; then
      continue
    fi
    if ! is_reapable_leftover "$ppid" "$etime" "$cmd" "$state" "$cpu"; then
      continue
    fi
    kind="$(leftover_kind "$cmd" || true)"
    if is_zombie_state "$state" "$cmd"; then
      pcmd="$(/bin/ps -p "$ppid" -o command= 2>/dev/null || true)"
      if [[ -n "$pcmd" ]] && is_host_app "$pcmd"; then
        skipped=$((skipped + 1))
        log "reap skip zombie pid=$pid parent_host ppid=$ppid"
        continue
      fi
      if [[ -n "$pcmd" ]] && leftover_kind "$pcmd" >/dev/null && ! is_host_app "$pcmd"; then
        printf 'reap zombie-parent pid=%s of zombie=%s %s\n' "$ppid" "$pid" "$(humanize_proc "$pcmd")"
        if safe_kill_pid "$ppid" "reap-zombie-parent"; then
          kills=$((kills + 1))
        else
          skipped=$((skipped + 1))
        fi
        continue
      fi
      skipped=$((skipped + 1))
      log "reap skip zombie pid=$pid ppid=$ppid (cannot reap without host parent)"
      continue
    fi
    if is_server_kind "$kind" && server_has_clients "$pid"; then
      skipped=$((skipped + 1))
      log "reap skip in-use-server pid=$pid kind=$kind"
      continue
    fi
    printf 'reap pid=%s ppid=%s etime=%s cpu=%s %s\n' "$pid" "$ppid" "$etime" "$cpu" "$(humanize_proc "$cmd")"
    if safe_kill_pid "$pid" "reap-unused-leftover"; then
      kills=$((kills + 1))
    else
      skipped=$((skipped + 1))
    fi
  done < <(/bin/ps -axo pid=,ppid=,etime=,%cpu=,state=,command= 2>/dev/null | /usr/bin/awk '{
    pid=$1; ppid=$2; etime=$3; cpu=$4; state=$5;
    $1=""; $2=""; $3=""; $4=""; $5="";
    sub(/^  */, "", $0);
    printf "%s\t%s\t%s\t%s\t%s\t%s\n", pid, ppid, etime, cpu, state, $0
  }')
  printf 'reaped=%s skipped=%s dry_run=%s\n' "$kills" "$skipped" "$MG_DRY_RUN"
}

mark_never_index() {
  local d IFS=':'
  [[ -n "${MG_NEVER_INDEX_DIRS:-}" ]] || return 0
  for d in $MG_NEVER_INDEX_DIRS; do
    [[ -d "$d" ]] || continue
    if [[ ! -e "$d/.metadata_never_index" ]]; then
      : >"$d/.metadata_never_index"
      printf 'never_index %s\n' "$d"
    fi
  done
}

# User-space pressure relief for fseventsd. Never SIGKILLs fseventsd (root Apple daemon).
# git fsmonitor daemons are idle watchers; git respawns them on the next command.
cmd_relieve_fsevents() {
  local pid cmd kills=0
  load_config
  ensure_dirs
  mark_never_index
  while IFS= read -r pid; do
    [[ -z "$pid" ]] && continue
    cmd="$(/bin/ps -p "$pid" -o command= 2>/dev/null || true)"
    case "$cmd" in
      *"fsmonitor--daemon"*)
        printf 'relieve git-fsmonitor pid=%s\n' "$pid"
        if safe_kill_pid "$pid" "relieve-fsmonitor"; then
          kills=$((kills + 1))
        fi
        ;;
    esac
  done < <(/usr/bin/pgrep -f 'git fsmonitor--daemon run' 2>/dev/null || true)
  log "relieve-fsevents fsmonitor_killed=$kills dry_run=$MG_DRY_RUN"
  printf 'fsmonitor_stopped=%s dry_run=%s\n' "$kills" "$MG_DRY_RUN"
  printf 'fseventsd is Apple/root. launchd restarts it after: sudo killall fseventsd\n'
  if sudo -n /usr/bin/killall fseventsd 2>/dev/null; then
    printf 'fseventsd_restarted=1\n'
    log "relieve-fsevents restarted fseventsd via sudo -n"
  else
    printf 'fseventsd_restarted=0 (needs: sudo killall fseventsd)\n'
  fi
}

PENDING_CLOSE_DIR="${MEMORY_GUARD_PENDING_CLOSE:-$STATE_DIR/pending-close}"
SNOOZE_DIR="${MEMORY_GUARD_SNOOZE_DIR:-$STATE_DIR/session-snooze}"
CLOSING_FILE="${MEMORY_GUARD_CLOSING_FILE:-$STATE_DIR/SESSION_CLOSING.md}"

session_key() {
  local pid="$1" lstart="$2"
  printf '%s\n' "${pid}-$(printf '%s' "$lstart" | /usr/bin/tr -cd 'A-Za-z0-9' | /usr/bin/cut -c1-40)"
}

is_snoozed() {
  local key="$1" now until
  now="$(date +%s)"
  [[ -f "$SNOOZE_DIR/$key" ]] || return 1
  until="$(cat "$SNOOZE_DIR/$key" 2>/dev/null || echo 0)"
  [[ "$until" =~ ^[0-9]+$ ]] || return 1
  (( now < until ))
}

write_snooze() {
  local key="$1"
  mkdir -p "$SNOOZE_DIR"
  printf '%s\n' "$(( $(date +%s) + MG_SESSION_SNOOZE_SEC ))" >"$SNOOZE_DIR/$key"
}

write_closing_notice() {
  local label="$1" pid="$2" days="$3" deadline="$4" when
  when="$(date -r "$deadline" '+%Y-%m-%d %H:%M' 2>/dev/null || echo "$deadline")"
  mkdir -p "$(dirname "$CLOSING_FILE")"
  cat >"$CLOSING_FILE" <<EOF
# Sesión a punto de cerrarse

- App: ${label}
- PID: ${pid}
- Lleva: ${days} días
- Cierre acordado: ${when}

El operador aceptó cerrar esta sesión. Antes del cierre: commit, documenta,
actualiza agent-fleet si aplica, y deja un handoff. No esperes a que te lo
pidan otra vez. El trabajo no se debe perder.
EOF
  notify "Cierre de ${label} a las ${when}. Pide a los agentes: commit, documentar, handoff." "Memory Guard · sesión"
}

session_close_pid() {
  local pid="$1" expect_lstart="$2" reason="$3"
  local live
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  (( pid > 1 )) || return 1
  live="$(/bin/ps -p "$pid" -o lstart= 2>/dev/null || true)"
  live="$(printf '%s' "$live" | /usr/bin/sed 's/^ *//;s/ *$//')"
  if [[ -z "$live" || "$live" != "$expect_lstart" ]]; then
    log "session-close skip pid=$pid lstart mismatch"
    return 1
  fi
  if [[ "$MG_DRY_RUN" == "1" ]]; then
    log "DRY session-close pid=$pid reason=$reason"
    return 0
  fi
  log "session-close SIGTERM pid=$pid reason=$reason"
  /bin/kill -TERM "$pid" 2>/dev/null || return 1
  return 0
}

process_pending_closes() {
  local f pid lstart label deadline now live
  now="$(date +%s)"
  [[ -d "$PENDING_CLOSE_DIR" ]] || return 0
  for f in "$PENDING_CLOSE_DIR"/*; do
    [[ -f "$f" ]] || continue
    pid="$(/usr/bin/sed -n '1p' "$f")"
    deadline="$(/usr/bin/sed -n '2p' "$f")"
    label="$(/usr/bin/sed -n '3p' "$f")"
    lstart="$(/usr/bin/sed -n '4p' "$f")"
    [[ "$pid" =~ ^[0-9]+$ ]] || { rm -f "$f"; continue; }
    [[ "$deadline" =~ ^[0-9]+$ ]] || continue
    if (( now < deadline )); then
      continue
    fi
    live="$(/bin/ps -p "$pid" -o lstart= 2>/dev/null || true)"
    live="$(printf '%s' "$live" | /usr/bin/sed 's/^ *//;s/ *$//')"
    if [[ -n "$live" && "$live" == "$lstart" ]]; then
      notify "Cerrando ${label:-sesión} (pid ${pid}) tras el margen para commit/docs." "Memory Guard · sesión"
      session_close_pid "$pid" "$lstart" "operator-confirmed-month-old"
    fi
    rm -f "$f"
  done
  if [[ -d "$PENDING_CLOSE_DIR" ]] && ! ls "$PENDING_CLOSE_DIR"/* >/dev/null 2>&1; then
    rm -f "$CLOSING_FILE"
  fi
}

schedule_session_close() {
  local pid="$1" lstart="$2" label="$3" days="$4" grace_min="$5"
  local deadline key
  deadline=$(( $(date +%s) + grace_min * 60 ))
  key="$(session_key "$pid" "$lstart")"
  mkdir -p "$PENDING_CLOSE_DIR"
  printf '%s\n%s\n%s\n%s\n' "$pid" "$deadline" "$label" "$lstart" >"$PENDING_CLOSE_DIR/$key"
  write_closing_notice "$label" "$pid" "$days" "$deadline"
  log "session-close scheduled pid=$pid label=$label grace_min=$grace_min"
}

session_dialog() {
  local label="$1" days="$2" pid="$3"
  label="$(printf '%s' "$label" | /usr/bin/tr -d '"')"
  if [[ "$MG_SESSION_DIALOG" != "1" ]]; then
    printf '%s\n' "snooze"
    return 0
  fi
  /usr/bin/osascript 2>/dev/null <<EOF || printf '%s\n' "snooze"
try
  set r to display dialog "La sesión ${label} (pid ${pid}) lleva ${days} días abierta.

Si aceptas, los agentes tendrán un margen para commit, documentar y no perder trabajo. Después se cierra.

¿Cerrar esta sesión?" buttons {"Ahora no", "Sí, en 15 min", "Sí, en 1 hora"} default button "Ahora no" giving up after ${MG_SESSION_DIALOG_TIMEOUT}
  if gave up of r then
    return "snooze"
  end if
  set b to button returned of r
  if b is "Sí, en 15 min" then
    return "15"
  else if b is "Sí, en 1 hora" then
    return "60"
  else
    return "snooze"
  end if
on error
  return "snooze"
end try
EOF
}

list_month_sessions() {
  local pid ppid etime cmd days
  /bin/ps -axo pid=,ppid=,etime=,command= 2>/dev/null | while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"
    pid="${line%% *}"
    rest="${line#* }"
    rest="${rest#"${rest%%[![:space:]]*}"}"
    ppid="${rest%% *}"
    rest="${rest#* }"
    rest="${rest#"${rest%%[![:space:]]*}"}"
    etime="${rest%% *}"
    cmd="${rest#* }"
    cmd="${cmd#"${cmd%%[![:space:]]*}"}"
    is_session_host "$cmd" || continue
    days="$(session_days "$etime")"
    (( days >= 30 )) || continue
    printf '%s\t%s\t%s\t%s\t%s\n' "$pid" "$ppid" "$etime" "$days" "$(humanize_proc "$cmd")"
  done
}

PENDING_PROMPT_FILE="${MEMORY_GUARD_PENDING_PROMPT:-$STATE_DIR/pending-prompt.txt}"

# Daemon path: notify only. Never osascript (that is what made it "sometimes").
session_hygiene() {
  local pid ppid etime cmd days lstart key label
  [[ -d "$PENDING_CLOSE_DIR" ]] && ls "$PENDING_CLOSE_DIR"/* >/dev/null 2>&1 && return 0
  [[ -f "$PENDING_PROMPT_FILE" ]] && return 0
  if session_prompt_recent; then
    return 0
  fi
  while IFS=$'\t' read -r pid ppid etime days label; do
    [[ -z "$pid" ]] && continue
    cmd="$(/bin/ps -p "$pid" -o command= 2>/dev/null || true)"
    session_should_prompt "$ppid" "$cmd" || continue
    lstart="$(/bin/ps -p "$pid" -o lstart= 2>/dev/null || true)"
    lstart="$(printf '%s' "$lstart" | /usr/bin/sed 's/^ *//;s/ *$//')"
    key="$(session_key "$pid" "$lstart")"
    if is_snoozed "$key"; then
      continue
    fi
    mkdir -p "$(dirname "$PENDING_PROMPT_FILE")"
    printf '%s\n%s\n%s\n%s\n%s\n' "$pid" "$ppid" "$days" "$label" "$lstart" >"$PENDING_PROMPT_FILE"
    mark_session_prompt
    log "session-hygiene pending pid=$pid days=$days label=$label"
    notify "${label} lleva ${days} días. Confirma con: memory-guard review-sessions" "Memory Guard · sesión"
    return 0
  done < <(list_month_sessions)
}

cmd_review_sessions() {
  local pid ppid days label lstart key ans cmd
  load_config
  ensure_dirs
  if [[ -f "$PENDING_PROMPT_FILE" ]]; then
    pid="$(/usr/bin/sed -n '1p' "$PENDING_PROMPT_FILE")"
    ppid="$(/usr/bin/sed -n '2p' "$PENDING_PROMPT_FILE")"
    days="$(/usr/bin/sed -n '3p' "$PENDING_PROMPT_FILE")"
    label="$(/usr/bin/sed -n '4p' "$PENDING_PROMPT_FILE")"
    lstart="$(/usr/bin/sed -n '5p' "$PENDING_PROMPT_FILE")"
    cmd="$(/bin/ps -p "$pid" -o command= 2>/dev/null || true)"
    if [[ -z "$cmd" ]] || ! session_should_prompt "$ppid" "$cmd"; then
      rm -f "$PENDING_PROMPT_FILE"
      echo "pending prompt stale; nothing to review"
      return 0
    fi
  else
    echo "no pending month-old session (daemon notifies when there is one)"
    cmd_sessions
    return 0
  fi
  key="$(session_key "$pid" "$lstart")"
  ans="$(session_dialog "$label" "$days" "$pid")"
  ans="$(printf '%s' "$ans" | /usr/bin/tr -d '\r')"
  rm -f "$PENDING_PROMPT_FILE"
  case "$ans" in
    15) schedule_session_close "$pid" "$lstart" "$label" "$days" 15 ;;
    60) schedule_session_close "$pid" "$lstart" "$label" "$days" 60 ;;
    *)
      write_snooze "$key"
      notify "De acuerdo: ${label} sigue. Aviso de nuevo en 7 días." "Memory Guard · sesión"
      echo "snoozed $label pid=$pid"
      ;;
  esac
}

cmd_sessions() {
  load_config
  printf '%-8s %-8s %-12s %-6s %s\n' "PID" "PPID" "ETIME" "DAYS" "SESSION"
  list_month_sessions | while IFS=$'\t' read -r pid ppid etime days label; do
    printf '%-8s %-8s %-12s %-6s %s\n' "$pid" "$ppid" "$etime" "$days" "$label"
  done
  if [[ -f "$CLOSING_FILE" ]]; then
    echo
    echo "pending:"
    cat "$CLOSING_FILE"
  fi
}

cmd_status() {
  ensure_dirs
  if [[ -f "$STATUS_FILE" ]]; then
    cat "$STATUS_FILE"
  else
    echo "status=missing"
  fi
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "${pid:-}" ]] && /bin/kill -0 "$pid" 2>/dev/null; then
      echo "daemon=running pid=$pid"
    else
      echo "daemon=stale pid=${pid:-none}"
    fi
  else
    echo "daemon=not-tracked"
  fi
}

cmd_once() {
  load_config
  ensure_dirs
  sample_once >/dev/null
  cmd_status
}

acquire_run_lock() {
  local lockdir="$STATE_DIR/run.lock" old cmd
  if mkdir "$lockdir" 2>/dev/null; then
    printf '%s\n' "$$" >"$lockdir/pid"
    printf '%s\n' "$$" >"$PID_FILE"
    return 0
  fi
  old="$(cat "$lockdir/pid" 2>/dev/null || true)"
  if [[ "$old" =~ ^[0-9]+$ ]] && /bin/kill -0 "$old" 2>/dev/null; then
    cmd="$(/bin/ps -p "$old" -o command= 2>/dev/null || true)"
    case "$cmd" in
      *memory-guard.sh*run*)
        log "already running pid=$old"
        return 1
        ;;
    esac
  fi
  rm -rf "$lockdir"
  mkdir "$lockdir" 2>/dev/null || return 1
  printf '%s\n' "$$" >"$lockdir/pid"
  printf '%s\n' "$$" >"$PID_FILE"
  return 0
}

release_run_lock() {
  rm -rf "$STATE_DIR/run.lock"
}

cmd_run() {
  load_config
  ensure_dirs
  if ! acquire_run_lock; then
    exit 0
  fi
  trap 'release_run_lock' EXIT
  log "memory-guard v$VERSION start pid=$$ auto_kill=$MG_AUTO_KILL dry_run=$MG_DRY_RUN"
  notify "Memory Guard activo (v$VERSION)" "Memory Guard"
  local level interval
  while true; do
    level="$(sample_once)"
    process_pending_closes
    if [[ "$level" != "emergency" ]]; then
      session_hygiene
    fi
    interval="$(interval_for_level "$level")"
    /bin/sleep "$interval"
  done
}

# Pure classifier contract. No ps, no osascript. fail=1 on mismatch.
prove_classifier() {
  local fail=0 line ppid etime cmd state cpu expect got
  while IFS= read -r line; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    ppid="${line%%|*}"; rest="${line#*|}"
    etime="${rest%%|*}"; rest="${rest#*|}"
    cmd="${rest%%|*}"; rest="${rest#*|}"
    state="${rest%%|*}"; rest="${rest#*|}"
    cpu="${rest%%|*}"; expect="${rest#*|}"
    if is_reapable_leftover "$ppid" "$etime" "$cmd" "$state" "$cpu"; then
      got=yes
    else
      got=no
    fi
    if [[ "$got" != "$expect" ]]; then
      echo "FAIL prove reap ppid=$ppid etime=$etime expect=$expect got=$got cmd=$cmd"
      fail=1
    fi
  done <<'CASES'
1|01-16:00:00|flutter_tester --disable-vm-service|S|0|yes
25033|01-16:00:00|flutter_tester --disable-vm-service|S|0|no
1|00:05:00|flutter_tester --disable-vm-service|S|0|no
1|01-16:00:00|flutter_tester --disable-vm-service|S|9.5|no
1|02:00:00|/usr/bin/git fsmonitor--daemon run --detach|S|0|yes
1|00:10:00|/usr/bin/git fsmonitor--daemon run --detach|S|0|no
1|08-01:16:26|/opt/homebrew/Cellar/php/8.5.9/bin/php -d display_errors=0 -S 127.0.0.1:61505 -t /tmp/site|S|0|yes
9|08-01:16:26|/opt/homebrew/Cellar/php/8.5.9/bin/php -d display_errors=0 -S 127.0.0.1:61505 -t /tmp/site|S|0|no
1|19-01:00:00|next-server (v16.2.11)|S|0|yes
48202|19-01:00:00|next-server (v16.2.11)|S|0|no
1|01:12:00|Python -m http.server 8801 --bind 127.0.0.1|S|0|yes
1|00:12:00|Python -m http.server 8801 --bind 127.0.0.1|S|0|no
176|10-21:24:31|npm exec firebase-tools@latest mcp|S|0|no
1|10-21:24:31|npm exec firebase-tools@latest mcp|S|0|no
1|18-17:00:00|/Applications/Warp.app/Contents/MacOS/stable --finish-update|S|5|no
1|37-16:00:00|/System/Library/Frameworks/CoreServices.framework/Versions/A/Support/fseventsd|S|80|no
1|02-00:00:00|head -40|Z|0|yes
9665|10-00:00:00|<defunct>|Z|0|no
CASES
  if session_should_prompt 6186 "/Applications/Warp.app/Contents/MacOS/stable"; then :; else echo "FAIL prove prompt warp"; fail=1; fi
  if session_should_prompt 6186 "claude --dangerously-skip-permissions"; then echo "FAIL prove prompt claude-in-warp"; fail=1; fi
  if session_should_prompt 1 "claude --dangerously-skip-permissions"; then :; else echo "FAIL prove prompt orphan claude"; fail=1; fi
  if session_should_prompt 1 "npm exec firebase-tools@latest mcp"; then echo "FAIL prove prompt mcp"; fail=1; fi
  if is_session_container "claude --dangerously-skip-permissions"; then echo "FAIL prove container claude"; fail=1; fi
  if is_host_app "/Applications/Warp.app/Contents/MacOS/stable --finish-update"; then :; else echo "FAIL prove host warp"; fail=1; fi
  return "$fail"
}

cmd_prove() {
  if prove_classifier; then
    echo "memory-guard prove OK"
    return 0
  fi
  echo "memory-guard prove FAILED"
  return 1
}

cmd_selftest() {
  local fail=0
  local got

  got="$(printf '%s\n' 'System-wide memory free percentage: 42%' | parse_free_pct)"
  [[ "$got" == "42" ]] || { echo "FAIL free_pct got=$got"; fail=1; }

  got="$(printf '%s\n' 'vm.swapusage: total = 2048.00M  used = 512.50M  free = 1535.50M (encrypted)' | parse_swap_used_mb)"
  [[ "$got" == "513" || "$got" == "512" ]] || { echo "FAIL swap got=$got"; fail=1; }

  got="$(printf '%s\n' 'Pages occupied by compressor: 1024.' | parse_compressor_mb 16384)"
  # 1024 * 16384 / 1024 / 1024 = 16 MB
  [[ "$got" == "16" ]] || { echo "FAIL compressor got=$got"; fail=1; }

  MG_FREE_WARN=25 MG_FREE_CRIT=15 MG_FREE_EMERGENCY=8
  MG_SWAP_WARN_MB=2048 MG_SWAP_CRIT_MB=6144 MG_SWAP_EMERGENCY_MB=12288
  MG_COMPRESSOR_WARN_MB=12288 MG_COMPRESSOR_CRIT_MB=20480
  MG_PROC_KILL_MB=6144 MG_CHROME_TOTAL_CRIT_MB=12288 MG_CHROME_TOTAL_EMERGENCY_MB=20480

  got="$(level_from_metrics 50 0 100 200 0)"
  [[ "$got" == "ok" ]] || { echo "FAIL level ok got=$got"; fail=1; }
  got="$(level_from_metrics 20 0 100 200 0)"
  [[ "$got" == "warn" ]] || { echo "FAIL level warn got=$got"; fail=1; }
  got="$(level_from_metrics 10 0 13000 200 0)"
  [[ "$got" == "crit" ]] || { echo "FAIL level crit got=$got"; fail=1; }
  got="$(level_from_metrics 5 9000 25000 8000 0)"
  [[ "$got" == "emergency" ]] || { echo "FAIL level emergency got=$got"; fail=1; }
  got="$(level_from_metrics 50 0 100 200 13000)"
  [[ "$got" == "warn" ]] || { echo "FAIL level compressor-warn got=$got"; fail=1; }
  # Swap hangover with healthy free must NOT be emergency
  got="$(level_from_metrics 40 11000 500 400 8000)"
  [[ "$got" == "crit" ]] || { echo "FAIL swap-hangover-not-emergency got=$got"; fail=1; }

  if is_protected_name WindowServer; then :; else echo "FAIL protected"; fail=1; fi
  if is_chrome_family "Google Chrome Helper (Renderer)"; then :; else echo "FAIL chrome"; fail=1; fi
  if is_chrome_main "Google Chrome"; then :; else echo "FAIL chrome_main"; fail=1; fi
  if is_chrome_main "Google Chrome Helper (Renderer)"; then echo "FAIL helper_as_main"; fail=1; fi
  if is_chrome_helper "Google Chrome Helper (Renderer)"; then :; else echo "FAIL chrome_helper"; fail=1; fi
  if is_chrome_helper "Google Chrome"; then echo "FAIL main_as_helper"; fail=1; fi

  if is_host_app "/Applications/Warp.app/Contents/MacOS/stable --finish-update"; then :; else echo "FAIL warp_finish_update_is_host"; fail=1; fi
  if is_host_app "/Applications/Warp.app/Contents/MacOS/stable"; then :; else echo "FAIL warp_is_host"; fail=1; fi
  if is_host_app "claude --dangerously-skip-permissions"; then :; else echo "FAIL claude_is_host"; fail=1; fi
  if is_host_app "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"; then :; else echo "FAIL chrome_main_is_host"; fail=1; fi
  if is_host_app "Google Chrome Helper (Renderer)"; then echo "FAIL helper_is_host"; fail=1; fi
  if is_host_app "flutter_tester --disable-vm-service"; then echo "FAIL tester_is_host"; fail=1; fi
  if is_host_app "npm exec firebase-tools@latest mcp"; then :; else echo "FAIL firebase_mcp_is_host"; fail=1; fi
  if is_host_app "npm exec @playwright/mcp@latest"; then :; else echo "FAIL playwright_mcp_is_host"; fail=1; fi
  if is_host_app "/Users/jorge/development/flutter/bin/cache/dart-sdk/bin/dart mcp-server --disable=pub"; then :; else echo "FAIL dart_mcp_is_host"; fail=1; fi
  if is_host_app "npm exec chrome-devtools-mcp@1.6.0 --isolated"; then :; else echo "FAIL chrome_mcp_is_host"; fail=1; fi

  if is_reapable_leftover 1 "01-16:00:00" "flutter_tester --disable-vm-service" S 0; then :; else echo "FAIL reap_orphan_tester"; fail=1; fi
  if is_reapable_leftover 1 "01:12:00" "Python -m http.server 8801 --bind 127.0.0.1" S 0; then :; else echo "FAIL reap_http_server"; fail=1; fi
  if is_reapable_leftover 1 "00:12:00" "Python -m http.server 8801 --bind 127.0.0.1" S 0; then echo "FAIL reap_young_http"; fail=1; fi
  if is_reapable_leftover 1 "18-17:00:00" "/Applications/Warp.app/Contents/MacOS/stable --finish-update" S 0; then echo "FAIL reap_warp"; fail=1; fi
  if is_reapable_leftover 48202 "00:01:00" "flutter_tester --disable-vm-service" S 0; then echo "FAIL reap_live_tester"; fail=1; fi
  if is_reapable_leftover 1 "01-16:00:00" "flutter_tester --disable-vm-service" S 12.0; then echo "FAIL reap_busy_tester"; fail=1; fi
  if is_reapable_leftover 1 "19-01:00:00" "next-server (v16.2.11)" S 0; then :; else echo "FAIL reap_old_next"; fail=1; fi
  if is_reapable_leftover 1 "08-01:16:26" "/opt/homebrew/Cellar/php/8.5.9/bin/php -d display_errors=0 -S 127.0.0.1:61505 -t /tmp/site" S 0; then :; else echo "FAIL reap_php"; fail=1; fi
  if is_reapable_leftover 9 "08-01:16:26" "/opt/homebrew/Cellar/php/8.5.9/bin/php -d display_errors=0 -S 127.0.0.1:61505 -t /tmp/site" S 0; then echo "FAIL reap_php_with_parent"; fail=1; fi
  if is_reapable_leftover 176 "10-21:24:31" "npm exec firebase-tools@latest mcp" S 0; then echo "FAIL reap_firebase_mcp"; fail=1; fi
  if is_reapable_leftover 72694 "01-00:00:00" "flutter_tester --disable-vm-service" S 0; then echo "FAIL reap_tester_with_parent"; fail=1; fi
  if is_reapable_leftover 1 "02:00:00" "/usr/bin/git fsmonitor--daemon run --detach" S 0; then :; else echo "FAIL reap_fsmonitor_orphan"; fail=1; fi
  if is_reapable_leftover 1 "00:10:00" "/usr/bin/git fsmonitor--daemon run --detach" S 0; then echo "FAIL reap_fsmonitor_young"; fail=1; fi
  if is_reapable_leftover 1 "37-00:00:00" "/System/Library/Frameworks/CoreServices.framework/Versions/A/Support/fseventsd" S 80; then echo "FAIL reap_fseventsd"; fail=1; fi
  if is_reapable_leftover 1 "02-00:00:00" "head -40" Z 0; then :; else echo "FAIL reap_zombie_pipe"; fail=1; fi
  if is_reapable_leftover 9665 "10-00:00:00" "<defunct>" Z 0; then echo "FAIL reap_cursor_zombie"; fail=1; fi

  got="$(etime_seconds "18-17:02:53")"
  # 18*86400 + 17*3600 + 2*60 + 53 = 1616573
  [[ "$got" == "1616573" ]] || { echo "FAIL etime_seconds got=$got"; fail=1; }
  got="$(etime_seconds "08-01:16:26")"
  # 8*86400 + 1*3600 + 16*60 + 26 = 695786
  [[ "$got" == "695786" ]] || { echo "FAIL etime_seconds_leading_zero got=$got"; fail=1; }
  got="$(etime_seconds "31:35")"
  [[ "$got" == "1895" ]] || { echo "FAIL etime_mmss got=$got"; fail=1; }
  got="$(etime_seconds "01:12:00")"
  [[ "$got" == "4320" ]] || { echo "FAIL etime_hhmmss got=$got"; fail=1; }

  got="$(printf '%s\n' '1200|99|/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Helpers/Google Chrome Helper (Renderer)' | format_offenders 1)"
  [[ "$got" == "Chrome (pestaña) está usando 1.2 GB" ]] || { echo "FAIL format_offenders got=$got"; fail=1; }
  got="$(humanize_proc "/Applications/Warp.app/Contents/MacOS/stable --finish-update")"
  [[ "$got" == "Warp" ]] || { echo "FAIL humanize_warp got=$got"; fail=1; }

  if is_session_host "/Applications/Warp.app/Contents/MacOS/stable --finish-update"; then :; else echo "FAIL session_warp"; fail=1; fi
  if is_session_container "/Applications/Warp.app/Contents/MacOS/stable --finish-update"; then :; else echo "FAIL container_warp"; fail=1; fi
  if is_session_container "claude --dangerously-skip-permissions"; then echo "FAIL container_claude"; fail=1; fi
  if is_session_host "claude --dangerously-skip-permissions"; then :; else echo "FAIL session_claude"; fail=1; fi
  if is_session_host "/Users/jorge/.opencode/bin/opencode --auto"; then :; else echo "FAIL session_opencode"; fail=1; fi
  if is_session_host "npm exec firebase-tools@latest mcp"; then echo "FAIL session_mcp"; fail=1; fi
  if is_session_host "/System/Library/Frameworks/CoreServices.framework/Versions/A/Support/fseventsd"; then echo "FAIL session_fseventsd"; fail=1; fi
  if is_session_host "flutter_tester --disable-vm-service"; then echo "FAIL session_tester"; fail=1; fi
  if session_should_prompt 1 "claude --dangerously-skip-permissions"; then :; else echo "FAIL prompt_orphan_claude"; fail=1; fi
  if session_should_prompt 6186 "claude --dangerously-skip-permissions"; then echo "FAIL prompt_claude_in_warp"; fail=1; fi
  if session_should_prompt 6186 "/Applications/Warp.app/Contents/MacOS/stable"; then :; else echo "FAIL prompt_warp"; fail=1; fi
  got="$(session_days "32-01:00:00")"
  [[ "$got" == "32" ]] || { echo "FAIL session_days got=$got"; fail=1; }
  got="$(session_days "10-00:00:00")"
  [[ "$got" == "10" ]] || { echo "FAIL session_days_10 got=$got"; fail=1; }

  if ! prove_classifier; then
    fail=1
  fi

  got="$(interval_for_level crit)"
  [[ "$got" == "$MG_INTERVAL_OK" ]] || { echo "FAIL interval_crit_is_fixed got=$got"; fail=1; }

  if (( fail == 0 )); then
    echo "memory-guard selftest OK"
    return 0
  fi
  echo "memory-guard selftest FAILED"
  return 1
}

usage() {
  cat <<'EOF'
Usage: memory-guard.sh <run|once|status|offenders|reap|selftest|version>
  run               daemon loop (launchd). No osascript. One instance.
  once              single sample + print status
  status            print last status.env
  offenders         top RAM processes with class
  reap              leftovers only (orphan+idle+>=1h). never hosts/MCP/live tests
  relieve-fsevents  stop idle git fsmonitor; never kills Apple fseventsd
  sessions          list Warp/agent sessions older than 30 days
  review-sessions   yes/no dialog for a pending month-old session (only CLI)
  prove             classifier contract (no live processes)
  selftest          parsers + prove
  version           print version
Env/config: ~/.config/omnimon/memory-guard.conf
State/logs: ~/.local/state/omnimon-memory-guard/
EOF
}

main() {
  local cmd="${1:-run}"
  case "$cmd" in
    run) cmd_run ;;
    once) cmd_once ;;
    status) cmd_status ;;
    offenders|top) cmd_offenders ;;
    reap) cmd_reap ;;
    relieve-fsevents|relieve) cmd_relieve_fsevents ;;
    sessions) cmd_sessions ;;
    review-sessions|review) cmd_review_sessions ;;
    prove) cmd_prove ;;
    selftest) cmd_selftest ;;
    version|--version|-V) echo "$VERSION" ;;
    -h|--help|help) usage ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
}

# Allow sourcing for tests
if [[ "${MEMORY_GUARD_LIB:-0}" == "1" ]]; then
  return 0 2>/dev/null || true
fi

main "$@"
