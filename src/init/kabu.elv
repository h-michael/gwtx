# kabu init (elvish)
if (has-external ::KABU::) {
  eval (::KABU:: completions elvish | slurp)
}

fn __kabu_cmd {|@args| ::KABU:: $@args }

fn kabu {|@args|
  if (and (eq (count $@args) 1) (eq $args[0] 'cd')) {
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    var dest = (::KABU:: path)
    if (not (eq $dest '')) {
      cd $dest
    }
  } elif (and (> (count $@args) 0) (eq $args[0] 'add')) {
    var cd_to = (try { ::KABU:: config get auto_cd.after_add } catch { "" })

    # Capture output while displaying it
    var tmpfile = (mktemp)
    try {
      ::KABU:: $@args 2>&1 | tee $tmpfile
    } catch {
      rm -f $tmpfile
      fail "kabu add failed"
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
    var cd_to = (try { ::KABU:: config get auto_cd.after_remove } catch { "" })
    var main_path = (try { ::KABU:: path --main } catch { "" })

    __kabu_cmd $@args

    # Check if current directory was removed
    if (not (path:is-dir $current_dir)) {
      if (eq $cd_to 'main') {
        if (not (eq $main_path '')) {
          cd $main_path
        }
      } elif (eq $cd_to 'select') {
        var dest = (::KABU:: path)
        if (not (eq $dest '')) {
          cd $dest
        }
      }
    }
  } else {
    __kabu_cmd $@args
  }
}

var __kabu_trust_root = ""
var __kabu_trust_config_mtime = ""
var __kabu_trust_state = ""

fn __kabu_trust_check {
  var root = (try { git rev-parse --show-toplevel } catch { "" })
  if (eq $root "") {
    set &__kabu_trust_root = ""
    set &__kabu_trust_config_mtime = ""
    set &__kabu_trust_state = ""
    return
  }

  # Get .kabu/config.yaml modification time
  var config_path = $root"/.kabu/config.yaml"
  var current_mtime = ""
  try {
    var mtime = (path:lstat $config_path)[mtime]
    set current_mtime = (echo $mtime | tr -d ' ')
  } catch {
    # If unable to get mtime, use empty string
  }

  # Invalidate cache if repository changed or .kabu/config.yaml was modified
  if (or (not (eq $root $__kabu_trust_root)) (not (eq $current_mtime $__kabu_trust_config_mtime))) {
    set &__kabu_trust_root = $root
    set &__kabu_trust_config_mtime = $current_mtime
    set &__kabu_trust_state = ""
  }

  var trusted = (try { ::KABU:: trust --check $root; put $true } catch { put $false })
  if $trusted {
    set &__kabu_trust_state = trusted
    return
  }

  if (not (eq $__kabu_trust_state untrusted)) {
    if (eq $term:color true) {
      echo >&2 "\e[31mkabu: hooks in config file are not trusted. Run 'kabu trust' to review them.\e[0m"
    } else {
      echo >&2 "kabu: hooks in config file are not trusted. Run 'kabu trust' to review them."
    }
  }
  set &__kabu_trust_state = untrusted
}

edit:before-prompt = [__kabu_trust_check $@edit:before-prompt]
edit:before-chdir = [__kabu_trust_check $@edit:before-chdir]
