# gwtx init (bash)
if command -v ::GWTX:: >/dev/null 2>&1; then
  source <(::GWTX:: completions bash)
fi

__gwtx_cmd() {
  ::GWTX:: "$@"
}

gwtx() {
  if [ "${1:-}" = "switch" ]; then
    local dest
    dest=$(__gwtx_cmd path) || return $?
    if [ -n "$dest" ]; then
      builtin cd "$dest"
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

  config_path="$root/.gwtx.yaml"

  # Get .gwtx.yaml mtime (Linux: stat -c '%Y', macOS: stat -f '%m')
  current_mtime=$(stat -c '%Y' "$config_path" 2>/dev/null || stat -f '%m' "$config_path" 2>/dev/null || echo "")

  # Invalidate cache if repository changed or .gwtx.yaml was modified
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
      printf '\033[31m%s\033[0m\n' "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    else
      printf '%s\n' "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
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
