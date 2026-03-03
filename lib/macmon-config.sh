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
        "$HOME/.config/macmon/"*|"${MACMON_HOME:-}/config/"*) return 0 ;;
        *) return 1 ;;
    esac
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
    local config_file="${1:-}"
    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"
    local validated_default=""
    local validated_user=""

    # Load defaults first, then user overrides
    if validated_default=$(_validated_config_path "$default_config"); then
        _parse_yaml "$validated_default"
    fi
    if [[ -n "$config_file" ]]; then
        if validated_user=$(_validated_config_path "$config_file"); then
            if _config_has_tab_indentation "$validated_user"; then
                echo "macmon: WARNING: config contains tab indentation, using safe defaults" >&2
                export MACMON_CFG_CONFIG_ERROR="tabs_in_yaml"
            elif ! _validate_custom_processes_block "$validated_user"; then
                echo "macmon: WARNING: invalid custom_processes syntax, using safe defaults" >&2
                export MACMON_CFG_CONFIG_ERROR="invalid_custom_processes"
            else
                _parse_yaml "$validated_user"
                export MACMON_CFG_CONFIG_ERROR=""
            fi
        elif [[ -f "$config_file" ]]; then
            echo "macmon: WARNING: blocked unsafe config path: $config_file" >&2
            export MACMON_CFG_CONFIG_ERROR="unsafe_config_path"
        fi
    fi

    _macmon_config_loaded=1
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
    # Return cache if already parsed
    if [[ "$_custom_processes_loaded" -eq 1 && -n "$_custom_processes_cache" ]]; then
        printf '%s\n' "$_custom_processes_cache"
        return 0
    fi

    local config_file="${MACMON_CONFIG:-}"
    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"
    local file=""

    # Use user config if it exists and defines custom_processes, otherwise default.
    # Both paths go through validation to avoid traversal/path injection.
    if [[ -n "$config_file" && -f "$config_file" ]]; then
        local validated_user
        validated_user=$(_validated_config_path "$config_file" || true)
        if [[ -n "$validated_user" ]] && grep -qE '^[[:space:]]*custom_processes:[[:space:]]*$' "$validated_user" 2>/dev/null && _validate_custom_processes_block "$validated_user" && ! _config_has_tab_indentation "$validated_user"; then
            file="$validated_user"
        fi
    fi
    if [[ -z "$file" && -f "$default_config" ]]; then
        file=$(_validated_config_path "$default_config" || true)
    fi

    [[ -n "$file" && -f "$file" ]] || return 1

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
        printf '%s\n' "$results"
        return 0
    fi
    return 1
}

# Invalidate custom_processes cache (called on config reload)
macmon_invalidate_custom_processes_cache() {
    _custom_processes_cache=""
    _custom_processes_loaded=0
}
