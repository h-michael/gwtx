# gwtx init (elvish)
if (has-external ::GWTX::) {
  eval (::GWTX:: completions elvish | slurp)
}

fn __gwtx_cmd {|@args| ::GWTX:: $@args }

fn gwtx {|@args|
  if (and (gt (count $@args) 0) (eq $args[0] 'switch')) {
    var dest = (::GWTX:: path)
    if (not (eq $dest '')) {
      cd $dest
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

  # Get .gwtx.yaml modification time
  var config_path = $root"/.gwtx.yaml"
  var current_mtime = ""
  try {
    var mtime = (path:lstat $config_path)[mtime]
    set current_mtime = (echo $mtime | tr -d ' ')
  } catch {
    # If unable to get mtime, use empty string
  }

  # Invalidate cache if repository changed or .gwtx.yaml was modified
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
      echo >&2 "\e[31mgwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them.\e[0m"
    } else {
      echo >&2 "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them."
    }
  }
  set &__gwtx_trust_state = untrusted
}

edit:before-prompt = [__gwtx_trust_check $@edit:before-prompt]
edit:before-chdir = [__gwtx_trust_check $@edit:before-chdir]
