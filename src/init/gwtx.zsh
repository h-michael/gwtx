# gwtx init (zsh)
if command -v ::GWTX:: >/dev/null 2>&1; then
  source <(::GWTX:: completions zsh)
fi

__gwtx_cmd() {
  ::GWTX:: "$@"
}

gwtx() {
  if [[ "${1:-}" == "cd" ]] && (( $# == 1 )); then
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    local dest
    dest=$(__gwtx_cmd path) || return $?
    if [[ -n "$dest" ]]; then
      builtin cd "$dest"
    fi
  elif [[ "${1:-}" == "add" ]]; then
    local cd_to
    cd_to=$(__gwtx_cmd config get auto_cd.after_add 2>/dev/null) || cd_to=""

    # Capture output while displaying it
    local tmpfile
    tmpfile=$(mktemp)
    __gwtx_cmd "$@" 2>&1 | tee "$tmpfile"
    local cmd_status=${pipestatus[1]}

    if [[ $cmd_status -eq 0 ]] && [[ "$cd_to" == "true" ]]; then
      local new_path
      new_path=$(tail -1 "$tmpfile")
      if [[ -d "$new_path" ]]; then
        builtin cd "$new_path"
      fi
    fi

    rm -f "$tmpfile"
    return $cmd_status
  elif [[ "${1:-}" == "remove" ]] || [[ "${1:-}" == "rm" ]]; then
    local current_dir="$PWD"
    # Get settings BEFORE removing (directory may not exist after)
    local cd_to
    cd_to=$(__gwtx_cmd config get auto_cd.after_remove 2>/dev/null) || cd_to=""
    local main_path
    main_path=$(__gwtx_cmd path --main 2>/dev/null) || main_path=""

    __gwtx_cmd "$@" || return $?

    # Check if current directory was removed
    if [[ ! -d "$current_dir" ]]; then
      case "$cd_to" in
        main)
          if [[ -n "$main_path" ]]; then
            builtin cd "$main_path"
          fi
          ;;
        select)
          local dest
          dest=$(__gwtx_cmd path) || return $?
          if [[ -n "$dest" ]]; then
            builtin cd "$dest"
          fi
          ;;
      esac
    fi
  else
    __gwtx_cmd "$@"
  fi
}

function __gwtx_trust_check() {
  local root config_path current_mtime
  root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    __gwtx_trust_root=""
    __gwtx_trust_config_mtime=""
    __gwtx_trust_state=""
    return
  }

  config_path="$root/.gwtx.yaml"

  # Get .gwtx.yaml mtime (Linux: stat -c '%Y', macOS: stat -f '%m')
  current_mtime=$(stat -c '%Y' "$config_path" 2>/dev/null || stat -f '%m' "$config_path" 2>/dev/null || echo "")

  # Invalidate cache if repository changed or .gwtx.yaml was modified
  if [[ "$root" != "$__gwtx_trust_root" ]] || \
     [[ "$current_mtime" != "$__gwtx_trust_config_mtime" ]]; then
    __gwtx_trust_root="$root"
    __gwtx_trust_config_mtime="$current_mtime"
    __gwtx_trust_state=""
  fi

  if ::GWTX:: trust --check "$root"; then
    __gwtx_trust_state="trusted"
    return
  fi

  if [[ "$__gwtx_trust_state" != "untrusted" ]]; then
    if [[ -t 2 ]]; then
      print -u2 $'%{\e[31m%}gwtx: hooks in .gwtx.yaml are not trusted. Run \\'gwtx trust\\' to review them.%{\e[0m%}'
    else
      print -u2 "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them."
    fi
  fi
  __gwtx_trust_state="untrusted"
}

function __gwtx_precmd() {
  __gwtx_trust_check
}

if typeset -a precmd_functions >/dev/null 2>&1; then
  if (( ! ${precmd_functions[(I)__gwtx_precmd]} )); then
    precmd_functions+=(__gwtx_precmd)
  fi
else
  precmd() { __gwtx_trust_check }
fi

function __gwtx_chpwd() {
  __gwtx_trust_check
}

if typeset -a chpwd_functions >/dev/null 2>&1; then
  if (( ! ${chpwd_functions[(I)__gwtx_chpwd]} )); then
    chpwd_functions+=(__gwtx_chpwd)
  fi
else
  chpwd() { __gwtx_trust_check }
fi
