# gwtx init (bash)
if command -v ::GWTX:: >/dev/null 2>&1; then
  source <(::GWTX:: completions bash)
fi

__gwtx_cmd() {
  ::GWTX:: "$@"
}

gwtx() {
  if [ "${1:-}" = "cd" ] && [ $# -eq 1 ]; then
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    local dest
    dest=$(__gwtx_cmd path) || return $?
    if [ -n "$dest" ]; then
      builtin cd "$dest"
    fi
  elif [ "${1:-}" = "add" ]; then
    local cd_to
    cd_to=$(__gwtx_cmd config get auto_cd.after_add 2>/dev/null) || cd_to=""

    # Capture output while displaying it
    local tmpfile
    tmpfile=$(mktemp)
    __gwtx_cmd "$@" 2>&1 | tee "$tmpfile"
    local cmd_status=${PIPESTATUS[0]}

    if [ $cmd_status -eq 0 ] && [ "$cd_to" = "true" ]; then
      local new_path
      new_path=$(tail -1 "$tmpfile")
      if [ -d "$new_path" ]; then
        builtin cd "$new_path"
      fi
    fi

    rm -f "$tmpfile"
    return $cmd_status
  elif [ "${1:-}" = "remove" ] || [ "${1:-}" = "rm" ]; then
    local current_dir="$PWD"
    # Get settings BEFORE removing (directory may not exist after)
    local cd_to
    cd_to=$(__gwtx_cmd config get auto_cd.after_remove 2>/dev/null) || cd_to=""
    local main_path
    main_path=$(__gwtx_cmd path --main 2>/dev/null) || main_path=""

    __gwtx_cmd "$@" || return $?

    # Check if current directory was removed
    if [ ! -d "$current_dir" ]; then
      case "$cd_to" in
        main)
          if [ -n "$main_path" ]; then
            builtin cd "$main_path"
          fi
          ;;
        select)
          local dest
          dest=$(__gwtx_cmd path) || return $?
          if [ -n "$dest" ]; then
            builtin cd "$dest"
          fi
          ;;
      esac
    fi
  else
    __gwtx_cmd "$@"
  fi
}

__gwtx_trust_check() {
  local root config_path current_mtime
  root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    __gwtx_trust_root=""
    __gwtx_trust_config_mtime=""
    __gwtx_trust_state=""
    return
  }

  config_path="$root/.gwtx/config.yaml"

  # Get .gwtx/config.yaml mtime (Linux: stat -c '%Y', macOS: stat -f '%m')
  current_mtime=$(stat -c '%Y' "$config_path" 2>/dev/null || stat -f '%m' "$config_path" 2>/dev/null || echo "")

  # Invalidate cache if repository changed or .gwtx/config.yaml was modified
  if [ "$root" != "$__gwtx_trust_root" ] || \
     [ "$current_mtime" != "$__gwtx_trust_config_mtime" ]; then
    __gwtx_trust_root="$root"
    __gwtx_trust_config_mtime="$current_mtime"
    __gwtx_trust_state=""
  fi

  if ::GWTX:: trust --check "$root"; then
    __gwtx_trust_state="trusted"
    return
  fi

  if [ "$__gwtx_trust_state" != "untrusted" ]; then
    if [ -t 2 ]; then
      printf '\033[31m%s\033[0m\n' "gwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    else
      printf '%s\n' "gwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    fi
  fi
  __gwtx_trust_state="untrusted"
}

__gwtx_prompt_hook() {
  local exit_code=$?
  __gwtx_trust_check
  return $exit_code
}

if [[ ";${PROMPT_COMMAND:-};" != *";__gwtx_prompt_hook;"* ]]; then
  if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == "declare -a"* ]]; then
    PROMPT_COMMAND=(__gwtx_prompt_hook "${PROMPT_COMMAND[@]}")
  elif [[ -n "${PROMPT_COMMAND:-}" ]]; then
    PROMPT_COMMAND="__gwtx_prompt_hook;${PROMPT_COMMAND}"
  else
    PROMPT_COMMAND="__gwtx_prompt_hook"
  fi
fi
