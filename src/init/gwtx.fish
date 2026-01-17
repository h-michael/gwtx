# gwtx init (fish)
if type -q ::GWTX::
  ::GWTX:: completions fish | source
end

function __gwtx_cmd
  ::GWTX:: $argv
end

function gwtx
  if test (count $argv) -gt 0; and test "$argv[1]" = "switch"
    set -l dest (__gwtx_cmd path)
    if test -n "$dest"
      builtin cd "$dest"
    end
  else
    __gwtx_cmd $argv
  end
end

function __gwtx_trust_check --on-event fish_prompt --on-variable PWD
  set -l root (git rev-parse --show-toplevel 2>/dev/null)
  if test -z "$root"
    set -g __gwtx_trust_root ""
    set -g __gwtx_trust_config_mtime ""
    set -g __gwtx_trust_state ""
    return
  end

  # Get .gwtx.yaml mtime (Linux: stat -c '%Y', macOS: stat -f '%m')
  set -l config_path "$root/.gwtx.yaml"

  # Skip if .gwtx.yaml doesn't exist
  if not test -f "$config_path"
    return
  end

  set -l current_mtime (stat -c '%Y' "$config_path" 2>/dev/null; or stat -f '%m' "$config_path" 2>/dev/null; or echo "")

  # Invalidate cache if repository changed or .gwtx.yaml was modified
  if test "$root" != "$__gwtx_trust_root"; or test "$current_mtime" != "$__gwtx_trust_config_mtime"
    set -g __gwtx_trust_root "$root"
    set -g __gwtx_trust_config_mtime "$current_mtime"
    set -g __gwtx_trust_state ""
  end

  # Run trust check and capture exit code
  if ::GWTX:: trust --check "$root" 2>/dev/null
    set -g __gwtx_trust_state trusted
    return
  end

  if test "$__gwtx_trust_state" != "untrusted"
    if test -t 2
      printf '\033[31m%s\033[0m\n' "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    else
      printf '%s\n' "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    end
  end
  set -g __gwtx_trust_state untrusted
end
