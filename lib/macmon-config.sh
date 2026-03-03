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
_expand_tilde() {
    local val="$1"
    case "$val" in
        # shellcheck disable=SC2088
        "~/"*) val="${HOME}/${val#\~/}" ;;
        "~")   val="$HOME" ;;
    esac
    printf '%s' "$val"
}

# Parse a flat YAML file into MACMON_CFG_SECTION_KEY variables
# Supports: key: value (nested one level with section prefix)
macmon_load_config() {
    local config_file="${1:-}"
    local default_config="${MACMON_HOME:-}/config/macmon.default.yaml"

    # Load defaults first, then user overrides
    if [[ -f "$default_config" ]]; then
        _parse_yaml "$default_config"
    fi
    if [[ -n "$config_file" && -f "$config_file" ]]; then
        _parse_yaml "$config_file"
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
