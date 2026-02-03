# gwtx init (elvish)
if (has-external ::GWTX::) {
  eval (::GWTX:: completions elvish | slurp)
}

fn __gwtx_cmd {|@args| ::GWTX:: $@args }

fn gwtx {|@args|
  if (and (eq (count $@args) 1) (eq $args[0] 'cd')) {
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    var dest = (::GWTX:: path)
    if (not (eq $dest '')) {
      cd $dest
    }
  } elif (and (> (count $@args) 0) (eq $args[0] 'add')) {
    var cd_to = (try { ::GWTX:: config get auto_cd.after_add } catch { "" })

    # Capture output while displaying it
    var tmpfile = (mktemp)
    try {
      ::GWTX:: $@args 2>&1 | tee $tmpfile
    } catch {
      rm -f $tmpfile
      fail "gwtx add failed"
    }

    if (eq $cd_to 'true') {
      var new_path = (tail -1 $tmpfile)
      if (and (not (eq $new_path '')) (path:is-dir $new_path)) {
        cd $new_path
      }
    }

    rm -f $tmpfile
  } elif (and (> (count $@args) 0) (or (eq $args[0] 'remove') (eq $args[0] 'rm'))) {
    var current_dir = $pwd
    # Get settings BEFORE removing (directory may not exist after)
    var cd_to = (try { ::GWTX:: config get auto_cd.after_remove } catch { "" })
    var main_path = (try { ::GWTX:: path --main } catch { "" })

    __gwtx_cmd $@args

    # Check if current directory was removed
    if (not (path:is-dir $current_dir)) {
      if (eq $cd_to 'main') {
        if (not (eq $main_path '')) {
          cd $main_path
        }
      } elif (eq $cd_to 'select') {
        var dest = (::GWTX:: path)
        if (not (eq $dest '')) {
          cd $dest
        }
      }
    }
  } else {
    __gwtx_cmd $@args
  }
}

var __gwtx_trust_root = ""
var __gwtx_trust_config_mtime = ""
var __gwtx_trust_state = ""

fn __gwtx_trust_check {
  var root = (try { git rev-parse --show-toplevel } catch { "" })
  if (eq $root "") {
    set &__gwtx_trust_root = ""
    set &__gwtx_trust_config_mtime = ""
    set &__gwtx_trust_state = ""
    return
  }

  # Get .gwtx/config.yaml modification time
  var config_path = $root"/.gwtx/config.yaml"
  var current_mtime = ""
  try {
    var mtime = (path:lstat $config_path)[mtime]
    set current_mtime = (echo $mtime | tr -d ' ')
  } catch {
    # If unable to get mtime, use empty string
  }

  # Invalidate cache if repository changed or .gwtx/config.yaml was modified
  if (or (not (eq $root $__gwtx_trust_root)) (not (eq $current_mtime $__gwtx_trust_config_mtime))) {
    set &__gwtx_trust_root = $root
    set &__gwtx_trust_config_mtime = $current_mtime
    set &__gwtx_trust_state = ""
  }

  var trusted = (try { ::GWTX:: trust --check $root; put $true } catch { put $false })
  if $trusted {
    set &__gwtx_trust_state = trusted
    return
  }

  if (not (eq $__gwtx_trust_state untrusted)) {
    if (eq $term:color true) {
      echo >&2 "\e[31mgwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them.\e[0m"
    } else {
      echo >&2 "gwtx: hooks in .gwtx/config.yaml are not trusted. Run 'gwtx trust' to review them."
    }
  }
  set &__gwtx_trust_state = untrusted
}

edit:before-prompt = [__gwtx_trust_check $@edit:before-prompt]
edit:before-chdir = [__gwtx_trust_check $@edit:before-chdir]
