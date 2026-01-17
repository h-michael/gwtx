# gwtx init (powershell)
if (Get-Command ::GWTX:: -ErrorAction SilentlyContinue) {
  & ::GWTX:: completions powershell | Out-String | Invoke-Expression
}

function __gwtx_cmd {
  param([Parameter(ValueFromRemainingArguments = $true)][object[]]$Args)
  & ::GWTX:: @Args
}

function gwtx {
  param([Parameter(ValueFromRemainingArguments = $true)][object[]]$Args)
  if ($Args.Count -gt 0 -and $Args[0] -eq "switch") {
    $dest = __gwtx_cmd path
    if ($dest) {
      Set-Location $dest
    }
  } else {
    __gwtx_cmd @Args
  }
}

function __gwtx_trust_check {
  try {
    $root = git rev-parse --show-toplevel 2>$null
  } catch {
    $global:__gwtx_trust_root = ""
    $global:__gwtx_trust_config_mtime = ""
    $global:__gwtx_trust_state = ""
    return
  }

  # Get .gwtx.yaml modification time
  $config_path = "$root\.gwtx.yaml"
  $current_mtime = ""
  try {
    if (Test-Path $config_path) {
      $current_mtime = (Get-Item $config_path).LastWriteTime.Ticks.ToString()
    }
  } catch {
    # If unable to get mtime, use empty string
  }

  # Invalidate cache if repository changed or .gwtx.yaml was modified
  if ($root -ne $global:__gwtx_trust_root -or $current_mtime -ne $global:__gwtx_trust_config_mtime) {
    $global:__gwtx_trust_root = $root
    $global:__gwtx_trust_config_mtime = $current_mtime
    $global:__gwtx_trust_state = ""
  }

  & ::GWTX:: trust --check $root | Out-Null
  if ($LASTEXITCODE -eq 0) {
    $global:__gwtx_trust_state = "trusted"
    return
  }

  if ($global:__gwtx_trust_state -ne "untrusted") {
    Write-Host "gwtx: hooks in .gwtx.yaml are not trusted. Run 'gwtx trust' to review them." -ForegroundColor Red
  }
  $global:__gwtx_trust_state = "untrusted"
}

function __gwtx_location_changed {
  __gwtx_trust_check
}

if ($ExecutionContext.SessionState.InvokeCommand.LocationChangedAction) {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction = [Delegate]::Combine(
    $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction,
    [EventHandler[LocationChangedEventArgs]] { param([object] $s, [LocationChangedEventArgs] $e) __gwtx_location_changed }
  )
} else {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction =
    [EventHandler[LocationChangedEventArgs]] { param([object] $s, [LocationChangedEventArgs] $e) __gwtx_location_changed }
}

if ($function:prompt) {
  $origPrompt = $function:prompt
  function prompt {
    __gwtx_trust_check
    & $origPrompt
  }
} else {
  function prompt {
    __gwtx_trust_check
    "PS $($executionContext.SessionState.Path.CurrentLocation)> "
  }
}
