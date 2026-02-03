# kabu init (powershell)
if (Get-Command ::KABU:: -ErrorAction SilentlyContinue) {
  & ::KABU:: completions powershell | Out-String | Invoke-Expression
}

function __kabu_cmd {
  param([Parameter(ValueFromRemainingArguments = $true)][object[]]$Args)
  & ::KABU:: @Args
}

function kabu {
  param([Parameter(ValueFromRemainingArguments = $true)][object[]]$Args)
  if ($Args.Count -eq 1 -and $Args[0] -eq "cd") {
    # Only use interactive path selection when "cd" has no additional arguments
    # If any arguments are provided (like --help), pass them to the command
    $dest = __kabu_cmd path
    if ($dest) {
      Set-Location $dest
    }
  } elseif ($Args.Count -ge 1 -and $Args[0] -eq "add") {
    $cdTo = ""
    try {
      $cdTo = __kabu_cmd config get auto_cd.after_add 2>$null
    } catch {}

    # Capture output while displaying it
    $tmpfile = [System.IO.Path]::GetTempFileName()
    __kabu_cmd @Args 2>&1 | Tee-Object -FilePath $tmpfile
    $cmdSuccess = $?

    if ($cmdSuccess -and $cdTo -eq "true") {
      $newPath = Get-Content $tmpfile | Select-Object -Last 1
      if ($newPath -and (Test-Path $newPath)) {
        Set-Location $newPath
      }
    }

    Remove-Item $tmpfile -ErrorAction SilentlyContinue
    if (-not $cmdSuccess) { return }
  } elseif ($Args.Count -ge 1 -and ($Args[0] -eq "remove" -or $Args[0] -eq "rm")) {
    $currentDir = $PWD.Path
    # Get settings BEFORE removing (directory may not exist after)
    $cdTo = ""
    $mainPath = ""
    try {
      $cdTo = __kabu_cmd config get auto_cd.after_remove 2>$null
      $mainPath = __kabu_cmd path --main 2>$null
    } catch {}

    __kabu_cmd @Args
    if (-not $?) { return }

    # Check if current directory was removed
    if (-not (Test-Path $currentDir)) {
      switch ($cdTo) {
        "main" {
          if ($mainPath) {
            Set-Location $mainPath
          }
        }
        "select" {
          $dest = __kabu_cmd path
          if ($dest) {
            Set-Location $dest
          }
        }
      }
    }
  } else {
    __kabu_cmd @Args
  }
}

function __kabu_trust_check {
  try {
    $root = git rev-parse --show-toplevel 2>$null
  } catch {
    $global:__kabu_trust_root = ""
    $global:__kabu_trust_config_mtime = ""
    $global:__kabu_trust_state = ""
    return
  }

  # Get .kabu/config.yaml modification time
  $config_path = "$root\.kabu\config.yaml"
  $current_mtime = ""
  try {
    if (Test-Path $config_path) {
      $current_mtime = (Get-Item $config_path).LastWriteTime.Ticks.ToString()
    }
  } catch {
    # If unable to get mtime, use empty string
  }

  # Invalidate cache if repository changed or .kabu/config.yaml was modified
  if ($root -ne $global:__kabu_trust_root -or $current_mtime -ne $global:__kabu_trust_config_mtime) {
    $global:__kabu_trust_root = $root
    $global:__kabu_trust_config_mtime = $current_mtime
    $global:__kabu_trust_state = ""
  }

  & ::KABU:: trust --check $root | Out-Null
  if ($LASTEXITCODE -eq 0) {
    $global:__kabu_trust_state = "trusted"
    return
  }

  if ($global:__kabu_trust_state -ne "untrusted") {
    Write-Host "kabu: hooks in .kabu/config.yaml are not trusted. Run 'kabu trust' to review them." -ForegroundColor Red
  }
  $global:__kabu_trust_state = "untrusted"
}

function __kabu_location_changed {
  __kabu_trust_check
}

if ($ExecutionContext.SessionState.InvokeCommand.LocationChangedAction) {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction = [Delegate]::Combine(
    $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction,
    [EventHandler[LocationChangedEventArgs]] { param([object] $s, [LocationChangedEventArgs] $e) __kabu_location_changed }
  )
} else {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction =
    [EventHandler[LocationChangedEventArgs]] { param([object] $s, [LocationChangedEventArgs] $e) __kabu_location_changed }
}

if ($function:prompt) {
  $origPrompt = $function:prompt
  function prompt {
    __kabu_trust_check
    & $origPrompt
  }
} else {
  function prompt {
    __kabu_trust_check
    "PS $($executionContext.SessionState.Path.CurrentLocation)> "
  }
}
