# gwtx init (fish)
if type -q ::GWTX::
  ::GWTX:: completions fish | source
end

function __gwtx_cmd
  ::GWTX:: $argv
end

function gwtx
  if test (count $argv) -eq 1; and test "$argv[1]" = "cd"
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    set -l dest (__gwtx_cmd path)
    if test -n "$dest"
      builtin cd "$dest"
    end
  else if test (count $argv) -ge 1; and test "$argv[1]" = "add"
    set -l cd_to (__gwtx_cmd config get auto_cd.after_add 2>/dev/null)

    # Capture output while displaying it
    set -l tmpfile (mktemp)
    __gwtx_cmd $argv 2>&1 | tee $tmpfile
    set -l cmd_status $pipestatus[1]

    if test $cmd_status -eq 0; and test "$cd_to" = "true"
      set -l new_path (tail -1 $tmpfile)
      if test -d "$new_path"
        builtin cd "$new_path"
      end
    end

    rm -f $tmpfile
    return $cmd_status
  else if test (count $argv) -ge 1; and begin test "$argv[1]" = "remove"; or test "$argv[1]" = "rm"; end
    set -l current_dir "$PWD"
    set -l cd_to (__gwtx_cmd config get auto_cd.after_remove 2>/dev/null)
    set -l main_path (__gwtx_cmd path --main 2>/dev/null)

    __gwtx_cmd $argv
    or return $status

    if not test -d "$current_dir"
      switch "$cd_to"
        case main
          if test -n "$main_path"
            builtin cd "$main_path"
          end
        case select
          set -l dest (__gwtx_cmd path)
          if test -n "$dest"
            builtin cd "$dest"
          end
      end
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

  # Get .gwtx/config.yaml mtime (Linux: stat -c '%Y', macOS: stat -f '%m')
  set -l config_path "$root/.gwtx/config.yaml"

  # Skip if .gwtx/config.yaml doesn't exist
  if not test -f "$config_path"
    return
  end

  set -l current_mtime (stat -c '%Y' "$config_path" 2>/dev/null; or stat -f '%m' "$config_path" 2>/dev/null; or echo "")

  # Invalidate cache if repository changed or .gwtx/config.yaml was modified
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
      printf '\033[31m%s\033[0m\n' "gwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    else
      printf '%s\n' "gwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them." 1>&2
    end
  end
  set -g __gwtx_trust_state untrusted
end
