# gwtx init (zsh)
if command -v ::GWTX:: >/dev/null 2>&1; then
  source <(::GWTX:: completions zsh)
fi

__gwtx_cmd() {
  ::GWTX:: "$@"
}

gwtx() {
  if [[ "${1:-}" == "switch" ]] && (( $# == 1 )); then
    # Only use interactive path selection when "switch" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    local dest
    dest=$(__gwtx_cmd path) || return $?
    if [[ -n "$dest" ]]; then
      builtin cd "$dest"
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
