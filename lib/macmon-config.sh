#!/usr/bin/env bash
# macmon-config.sh - Flat YAML config loader
# Sources config values as MACMON_CFG_* environment variables
# Compatible with bash 3.2+ (macOS default)

_macmon_config_loaded=0

# Uppercase helper (works on bash 3.2)
_to_upper() {
    printf '%s' "$1" | tr '[:lower:]' '[:upper:]'
}

# Expand ~ to $HOME in config values
# shellcheck disable=SC2088
_expand_tilde() {
    local val="$1"
    case "$val" in
        "~/"*) val="${HOME}/${val#\~/}" ;;
        "~")   val="$HOME" ;;
    esac
    printf '%s' "$val"
}

# Parse a flat YAML file into MACMON_CFG_SECTION_KEY variables
# Supports: key: value (nested one level with section prefix)
_resolve_existing_path() {
    local input="$1"
    [[ -n "$input" && -e "$input" ]] || return 1
    local dir base
    dir=$(cd "$(dirname "$input")" 2>/dev/null && pwd -P) || return 1
    base=$(basename "$input")
    printf '%s/%s' "$dir" "$base"
}

_is_allowed_config_path() {
    local resolved="$1"
    [[ "$resolved" == *.yaml || "$resolved" == *.yml ]] || return 1
    case "$resolved" in
        "$HOME/.config/macmon/"*|"$HOME/.config/macmon/profiles/"*|"${MACMON_HOME:-}/config/"*) return 0 ;;
        *) return 1 ;;
    esac
}

macmon_profiles_dir() {
    printf '%s\n' "$HOME/.config/macmon/profiles"
}

macmon_active_profile_file() {
    printf '%s\n' "$HOME/.config/macmon/active_profile"
}

_MACMON_CFG_LOCK_DEPTH=0
_MACMON_CFG_LOCK_DIR="${HOME}/.config/macmon/.macmon-config.lock"

_macmon_cfg_lock_acquire() {
    if (( _MACMON_CFG_LOCK_DEPTH > 0 )); then
        (( _MACMON_CFG_LOCK_DEPTH++ )) || true
        return 0
    fi
    mkdir -p "${HOME}/.config/macmon"
    local waited=0
    while ! mkdir "$_MACMON_CFG_LOCK_DIR" 2>/dev/null; do
        (( waited++ )) || true
        if (( waited > 100 )); then
            return 1
        fi
        sleep 0.05
    done
    _MACMON_CFG_LOCK_DEPTH=1
    return 0
}

_macmon_cfg_lock_release() {
    if (( _MACMON_CFG_LOCK_DEPTH <= 0 )); then
        return 0
    fi
    (( _MACMON_CFG_LOCK_DEPTH-- )) || true
    if (( _MACMON_CFG_LOCK_DEPTH == 0 )); then
        rmdir "$_MACMON_CFG_LOCK_DIR" 2>/dev/null || true
    fi
}

macmon_get_active_profile() {
    _macmon_cfg_lock_acquire || return 1
    local active_file
    active_file=$(macmon_active_profile_file)
    if [[ ! -f "$active_file" ]]; then
        _macmon_cfg_lock_release
        return 1
    fi
    local name
    name=$(tr -d '[:space:]' < "$active_file" 2>/dev/null || true)
    if [[ ! "$name" =~ ^[A-Za-z0-9._-]+$ ]]; then
        _macmon_cfg_lock_release
        return 1
    fi
    _macmon_cfg_lock_release
    printf '%s\n' "$name"
}

macmon_profile_path() {
    local profile="$1"
    [[ "$profile" =~ ^[A-Za-z0-9._-]+$ ]] || return 1
    local path
    path="$(macmon_profiles_dir)/${profile}.yaml"
    _validated_config_path "$path"
}

macmon_list_profiles() {
    local dir
    dir=$(macmon_profiles_dir)
    [[ -d "$dir" ]] || return 0
    local f
    for f in "$dir"/*.yaml "$dir"/*.yml; do
        [[ -f "$f" ]] || continue
        local base
        base=$(basename "$f")
        base="${base%.yaml}"
        base="${base%.yml}"
        [[ "$base" =~ ^[A-Za-z0-9._-]+$ ]] || continue
        printf '%s\n' "$base"
    done
}

macmon_set_active_profile() {
    _macmon_cfg_lock_acquire || return 1
    local profile="$1"
    local profile_path
    profile_path=$(macmon_profile_path "$profile" 2>/dev/null) || { _macmon_cfg_lock_release; return 1; }
    [[ -f "$profile_path" ]] || { _macmon_cfg_lock_release; return 1; }
    mkdir -p "$HOME/.config/macmon"
    local tmp active_file
    active_file=$(macmon_active_profile_file)
    tmp=$(mktemp "$HOME/.config/macmon/.active_profile.XXXXXX")
    printf '%s\n' "$profile" > "$tmp"
    mv -f "$tmp" "$active_file"
    _macmon_cfg_lock_release
}

MACMON_LOADED_CONFIG_PATH=""

macmon_resolve_config_file() {
    _macmon_cfg_lock_acquire || return 1
    local requested="${1:-}"
    local default_user="$HOME/.config/macmon/macmon.yaml"

    if [[ -n "$requested" ]]; then
        local validated_requested
        validated_requested=$(_validated_config_path "$requested" || true)
        if [[ -n "$validated_requested" ]]; then
            _macmon_cfg_lock_release
            printf '%s\n' "$validated_requested"
            return 0
        fi
    fi

    local active
    active=$(macmon_get_active_profile 2>/dev/null || true)
    if [[ -n "$active" ]]; then
        local profile_file
        profile_file=$(macmon_profile_path "$active" 2>/dev/null || true)
        if [[ -n "$profile_file" && -f "$profile_file" ]]; then
            _macmon_cfg_lock_release
            printf '%s\n' "$profile_file"
            return 0
        fi
    fi

    local validated_default_user
    validated_default_user=$(_validated_config_path "$default_user" || true)
    if [[ -n "$validated_default_user" && -f "$validated_default_user" ]]; then
        _macmon_cfg_lock_release
        printf '%s\n' "$validated_default_user"
        return 0
    fi

    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"
    local validated_default
    validated_default=$(_validated_config_path "$default_config" || true)
    if [[ -n "$validated_default" ]]; then
        _macmon_cfg_lock_release
        printf '%s\n' "$validated_default"
        return 0
    fi
    _macmon_cfg_lock_release
    return 1
}

macmon_get_loaded_config_path() {
    [[ -n "$MACMON_LOADED_CONFIG_PATH" ]] || return 1
    printf '%s\n' "$MACMON_LOADED_CONFIG_PATH"
}

_validated_config_path() {
    local candidate="$1"
    [[ -n "$candidate" ]] || return 1
    [[ "$candidate" != *".."* ]] || return 1
    local resolved
    resolved=$(_resolve_existing_path "$candidate") || return 1
    _is_allowed_config_path "$resolved" || return 1
    printf '%s\n' "$resolved"
}

_config_has_tab_indentation() {
    local file="$1"
    local line
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" == *$'\t'* ]] && return 0
    done < "$file"
    return 1
}

_validate_custom_processes_block() {
    local file="$1"
    local in_custom=0
    local seen_item=0
    local line val

    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" =~ ^[[:space:]]*$ ]] && continue

        if [[ "$line" =~ ^custom_processes:[[:space:]]*$ ]]; then
            in_custom=1
            continue
        fi

        if [[ "$in_custom" -eq 1 && "$line" =~ ^[a-zA-Z_] ]]; then
            break
        fi

        [[ "$in_custom" -eq 1 ]] || continue

        if [[ "$line" =~ ^[[:space:]]+-[[:space:]]+name:[[:space:]]*(.*)$ ]]; then
            val="${BASH_REMATCH[1]}"
            val="${val%%#*}"
            val="${val#\"}"; val="${val%\"}"
            val="${val#\'}"; val="${val%\'}"
            [[ -n "${val//[[:space:]]/}" ]] || return 1
            seen_item=1
            continue
        fi

        if [[ "$line" =~ ^[[:space:]]+-[[:space:]]+ ]]; then
            return 1
        fi

        if [[ "$line" =~ ^[[:space:]]+max_instances:[[:space:]]+(.*)$ ]]; then
            val="${BASH_REMATCH[1]}"; val="${val%%#*}"; val="${val//[[:space:]]/}"
            [[ "$val" =~ ^[0-9]+$ ]] || return 1
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]+max_ram_mb:[[:space:]]+(.*)$ ]]; then
            val="${BASH_REMATCH[1]}"; val="${val%%#*}"; val="${val//[[:space:]]/}"
            [[ "$val" =~ ^[0-9]+$ ]] || return 1
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]+max_cpu_percent:[[:space:]]+(.*)$ ]]; then
            val="${BASH_REMATCH[1]}"; val="${val%%#*}"; val="${val//[[:space:]]/}"
            [[ "$val" =~ ^[0-9]+$ ]] || return 1
            continue
        fi
    done < "$file"

    if grep -qE '^[[:space:]]*custom_processes:[[:space:]]*$' "$file" 2>/dev/null; then
        [[ "$seen_item" -eq 1 ]] || return 1
    fi
    return 0
}

macmon_load_config() {
    _macmon_cfg_lock_acquire || {
        export MACMON_CFG_CONFIG_ERROR="config_lock_timeout"
        return 1
    }
    local config_file="${1:-}"
    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"
    local validated_default=""
    local resolved_user=""
    MACMON_LOADED_CONFIG_PATH=""

    # Load defaults first, then user overrides
    if validated_default=$(_validated_config_path "$default_config"); then
        _parse_yaml "$validated_default"
    fi
    resolved_user=$(macmon_resolve_config_file "$config_file" || true)
    if [[ -n "$resolved_user" ]]; then
        if _config_has_tab_indentation "$resolved_user"; then
                echo "macmon: WARNING: config contains tab indentation, using safe defaults" >&2
                export MACMON_CFG_CONFIG_ERROR="tabs_in_yaml"
        elif ! _validate_custom_processes_block "$resolved_user"; then
                echo "macmon: WARNING: invalid custom_processes syntax, using safe defaults" >&2
                export MACMON_CFG_CONFIG_ERROR="invalid_custom_processes"
        else
            _parse_yaml "$resolved_user"
            MACMON_LOADED_CONFIG_PATH="$resolved_user"
            export MACMON_CFG_CONFIG_ERROR=""
        fi
    elif [[ -n "$config_file" && -f "$config_file" ]]; then
        echo "macmon: WARNING: blocked unsafe config path: $config_file" >&2
        export MACMON_CFG_CONFIG_ERROR="unsafe_config_path"
    fi

    [[ -n "$MACMON_LOADED_CONFIG_PATH" ]] || MACMON_LOADED_CONFIG_PATH="$validated_default"

    _macmon_config_loaded=1
    _macmon_cfg_lock_release
}

_parse_yaml() {
    local file="$1"
    local section=""
    local line key val

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" =~ ^[[:space:]]*$ ]] && continue

        # List item (- value) under a section
        if [[ "$line" =~ ^[[:space:]]+-[[:space:]]+(.*) ]]; then
            val="${BASH_REMATCH[1]}"
            val="${val%%#*}"    # strip inline comments
            val="${val%"${val##*[![:space:]]}"}"  # trim trailing
            if [[ -n "$section" ]]; then
                local list_var
                list_var="MACMON_CFG_$(_to_upper "$section")"
                local existing="${!list_var:-}"
                if [[ -n "$existing" ]]; then
                    export "$list_var=${existing}:${val}"
                else
                    export "$list_var=${val}"
                fi
            fi
            continue
        fi

        # Section header (no colon-value on same line, or value is empty)
        if [[ "$line" =~ ^([a-zA-Z_][a-zA-Z0-9_]*):[[:space:]]*$ ]]; then
            section="${BASH_REMATCH[1]}"
            continue
        fi

        # Indented key: value pair under a section
        if [[ "$line" =~ ^[[:space:]]+([a-zA-Z_][a-zA-Z0-9_]*):[[:space:]]+(.*) ]]; then
            key="${BASH_REMATCH[1]}"
            val="${BASH_REMATCH[2]}"
            val="${val%%#*}"    # strip inline comments
            val="${val%"${val##*[![:space:]]}"}"  # trim trailing
            if [[ -n "$section" ]]; then
                val=$(_expand_tilde "$val")
                export "MACMON_CFG_$(_to_upper "$section")_$(_to_upper "$key")=${val}"
            fi
            continue
        fi

        # Top-level key: value
        if [[ "$line" =~ ^([a-zA-Z_][a-zA-Z0-9_]*):[[:space:]]+(.*) ]]; then
            key="${BASH_REMATCH[1]}"
            val="${BASH_REMATCH[2]}"
            val="${val%%#*}"
            val="${val%"${val##*[![:space:]]}"}"
            val=$(_expand_tilde "$val")
            section=""
            export "MACMON_CFG_$(_to_upper "$key")=${val}"
            continue
        fi
    done < "$file"
}

# Get a config value with fallback default
macmon_cfg() {
    local key
    key="MACMON_CFG_$(_to_upper "$1")"
    local default="${2:-}"
    echo "${!key:-$default}"
}

# Check if config is loaded
macmon_config_loaded() {
    [[ "$_macmon_config_loaded" -eq 1 ]]
}

# --- Dynamic Process List Parser ---
# Parses custom_processes YAML block into a structured format.
# Returns colon-separated records: name:max_instances:max_ram_mb:max_cpu_percent
# Unset fields default to 0 (meaning "no limit").
# Usage: while IFS=: read -r name max_inst max_ram max_cpu; do ... done < <(macmon_get_custom_processes)
_custom_processes_cache=""
_custom_processes_loaded=0

macmon_get_custom_processes() {
    _macmon_cfg_lock_acquire || return 1
    # Return cache if already parsed
    if [[ "$_custom_processes_loaded" -eq 1 && -n "$_custom_processes_cache" ]]; then
        _macmon_cfg_lock_release
        printf '%s\n' "$_custom_processes_cache"
        return 0
    fi

    local config_file="${MACMON_CONFIG:-}"
    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"
    local file=""

    # Use resolved active config if it defines custom_processes, otherwise default.
    # Both paths go through validation to avoid traversal/path injection.
    local resolved
    resolved=$(macmon_resolve_config_file "$config_file" || true)
    if [[ -n "$resolved" && -f "$resolved" ]]; then
        local validated_user
        validated_user=$(_validated_config_path "$resolved" || true)
        if [[ -n "$validated_user" ]] && grep -qE '^[[:space:]]*custom_processes:[[:space:]]*$' "$validated_user" 2>/dev/null && _validate_custom_processes_block "$validated_user" && ! _config_has_tab_indentation "$validated_user"; then
            file="$validated_user"
        fi
    fi
    if [[ -z "$file" && -f "$default_config" ]]; then
        file=$(_validated_config_path "$default_config" || true)
    fi

    if [[ -z "$file" || ! -f "$file" ]]; then
        _macmon_cfg_lock_release
        return 1
    fi

    local in_custom=0
    local current_name="" max_inst="0" max_ram="0" max_cpu="0"
    local results=""
    local line val

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" =~ ^[[:space:]]*$ ]] && continue

        # Detect custom_processes section start
        if [[ "$line" =~ ^custom_processes:[[:space:]]*$ ]]; then
            in_custom=1
            continue
        fi

        # Detect end of section (non-indented line that isn't a list item)
        if [[ "$in_custom" -eq 1 && "$line" =~ ^[a-zA-Z] ]]; then
            # Flush last entry
            if [[ -n "$current_name" ]]; then
                results="${results}${current_name}:${max_inst}:${max_ram}:${max_cpu}"$'\n'
            fi
            in_custom=0
            continue
        fi

        [[ "$in_custom" -eq 1 ]] || continue

        # New list item: "  - name: value"
        if [[ "$line" =~ ^[[:space:]]+-[[:space:]]+name:[[:space:]]+(.*) ]]; then
            # Flush previous entry
            if [[ -n "$current_name" ]]; then
                results="${results}${current_name}:${max_inst}:${max_ram}:${max_cpu}"$'\n'
            fi
            val="${BASH_REMATCH[1]}"
            val="${val%%#*}"
            val="${val%"${val##*[![:space:]]}"}"
            # Strip surrounding quotes if present
            val="${val#\"}" ; val="${val%\"}"
            val="${val#\'}" ; val="${val%\'}"
            current_name="$val"
            max_inst="0" ; max_ram="0" ; max_cpu="0"
            continue
        fi

        # Sub-key under a list item: "    max_instances: N"
        if [[ "$line" =~ ^[[:space:]]+max_instances:[[:space:]]+(.*) ]]; then
            val="${BASH_REMATCH[1]}" ; val="${val%%#*}" ; val="${val%"${val##*[![:space:]]}"}"
            [[ "$val" =~ ^[0-9]+$ ]] && max_inst="$val"
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]+max_ram_mb:[[:space:]]+(.*) ]]; then
            val="${BASH_REMATCH[1]}" ; val="${val%%#*}" ; val="${val%"${val##*[![:space:]]}"}"
            [[ "$val" =~ ^[0-9]+$ ]] && max_ram="$val"
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]+max_cpu_percent:[[:space:]]+(.*) ]]; then
            val="${BASH_REMATCH[1]}" ; val="${val%%#*}" ; val="${val%"${val##*[![:space:]]}"}"
            [[ "$val" =~ ^[0-9]+$ ]] && max_cpu="$val"
            continue
        fi
    done < "$file"

    # Flush last entry
    if [[ "$in_custom" -eq 1 && -n "$current_name" ]]; then
        results="${results}${current_name}:${max_inst}:${max_ram}:${max_cpu}"$'\n'
    fi

    # Remove trailing newline
    results="${results%$'\n'}"
    _custom_processes_cache="$results"
    _custom_processes_loaded=1

    if [[ -n "$results" ]]; then
        _macmon_cfg_lock_release
        printf '%s\n' "$results"
        return 0
    fi
    _macmon_cfg_lock_release
    return 1
}

# Invalidate custom_processes cache (called on config reload)
macmon_invalidate_custom_processes_cache() {
    _custom_processes_cache=""
    _custom_processes_loaded=0
}
