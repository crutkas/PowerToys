// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;

using global::PowerToys.GPOWrapper;
using ManagedCommon;
using Microsoft.PowerToys.Settings.UI.Library.Helpers;
using Microsoft.PowerToys.Settings.UI.Library.Telemetry.Events;
using Microsoft.PowerToys.Settings.UI.Library.ViewModels.Commands;
using Microsoft.PowerToys.Telemetry;

namespace Microsoft.PowerToys.Settings.UI.ViewModels
{
    public partial class CmdNotFoundViewModel : Observable
    {
        public ButtonClickCommand InstallPowerShell7EventHandler => new ButtonClickCommand(InstallPowerShell7);

        public ButtonClickCommand InstallWinGetClientModuleEventHandler => new ButtonClickCommand(InstallWinGetClientModule);

        public ButtonClickCommand InstallModuleEventHandler => new ButtonClickCommand(InstallModule);

        public ButtonClickCommand UninstallModuleEventHandler => new ButtonClickCommand(UninstallModule);

        private GpoRuleConfigured _enabledGpoRuleConfiguration;
        private bool _moduleIsGpoEnabled;
        private bool _moduleIsGpoDisabled;
        private bool _isRequirementsCheckRunning;

        private sealed class CommandNotFoundRequirements
        {
            public CommandNotFoundRequirements(string outputLog, bool isPowerShell7Detected, bool isPowerShellPreviewDetected, string powerShellPreviewPath, bool isWinGetClientModuleDetected, bool isCommandNotFoundModuleInstalled)
            {
                OutputLog = outputLog;
                IsPowerShell7Detected = isPowerShell7Detected;
                IsPowerShellPreviewDetected = isPowerShellPreviewDetected;
                PowerShellPreviewPath = powerShellPreviewPath;
                IsWinGetClientModuleDetected = isWinGetClientModuleDetected;
                IsCommandNotFoundModuleInstalled = isCommandNotFoundModuleInstalled;
            }

            public string OutputLog { get; }

            public bool IsPowerShell7Detected { get; }

            public bool IsPowerShellPreviewDetected { get; }

            public string PowerShellPreviewPath { get; }

            public bool IsWinGetClientModuleDetected { get; }

            public bool IsCommandNotFoundModuleInstalled { get; }
        }

        public static string AssemblyDirectory
        {
            get
            {
                return Path.TrimEndingDirectorySeparator(AppContext.BaseDirectory);
            }
        }

        public CmdNotFoundViewModel()
        {
            InitializeEnabledValue();
        }

        private void InitializeEnabledValue()
        {
            _enabledGpoRuleConfiguration = GPOWrapper.GetConfiguredCmdNotFoundEnabledValue();
            _moduleIsGpoEnabled = _enabledGpoRuleConfiguration == GpoRuleConfigured.Enabled;
            _moduleIsGpoDisabled = _enabledGpoRuleConfiguration == GpoRuleConfigured.Disabled;

            // Update PATH environment variable to get pwsh.exe on further calls.
            Environment.SetEnvironmentVariable("PATH", (Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? string.Empty) + ";" + (Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.User) ?? string.Empty), EnvironmentVariableTarget.Process);
        }

        private string _commandOutputLog;

        public string CommandOutputLog
        {
            get => _commandOutputLog;
            set
            {
                if (_commandOutputLog != value)
                {
                    _commandOutputLog = value;
                    OnPropertyChanged(nameof(CommandOutputLog));
                }
            }
        }

        private bool _isPowerShell7Detected;

        private bool isPowerShellPreviewDetected;
        private string powerShellPreviewPath;

        public bool IsPowerShell7Detected
        {
            get => _isPowerShell7Detected;
            set
            {
                if (_isPowerShell7Detected != value)
                {
                    _isPowerShell7Detected = value;
                    OnPropertyChanged(nameof(IsPowerShell7Detected));
                }
            }
        }

        private bool _isWinGetClientModuleDetected;

        public bool IsWinGetClientModuleDetected
        {
            get => _isWinGetClientModuleDetected;
            set
            {
                if (_isWinGetClientModuleDetected != value)
                {
                    _isWinGetClientModuleDetected = value;
                    OnPropertyChanged(nameof(IsWinGetClientModuleDetected));
                }
            }
        }

        private bool _isCommandNotFoundModuleInstalled;

        public bool IsCommandNotFoundModuleInstalled
        {
            get => _isCommandNotFoundModuleInstalled;
            set
            {
                if (_isCommandNotFoundModuleInstalled != value)
                {
                    _isCommandNotFoundModuleInstalled = value;
                    OnPropertyChanged(nameof(IsCommandNotFoundModuleInstalled));
                }
            }
        }

        private bool _isCheckingRequirements = true;

        public bool IsCheckingRequirements
        {
            get => _isCheckingRequirements;
            private set
            {
                if (_isCheckingRequirements != value)
                {
                    _isCheckingRequirements = value;
                    OnPropertyChanged(nameof(IsCheckingRequirements));
                }
            }
        }

        public bool IsModuleGpoEnabled
        {
            get => _moduleIsGpoEnabled;
        }

        public bool IsModuleGpoDisabled
        {
            get => _moduleIsGpoDisabled;
        }

        public string RunPowerShellOrPreviewScript(string powershellExecutable, string powershellArguments, bool hidePowerShellWindow = false)
        {
            if (isPowerShellPreviewDetected)
            {
                return RunPowerShellScript(Path.Combine(powerShellPreviewPath, "pwsh-preview.cmd"), powershellArguments, hidePowerShellWindow);
            }
            else
            {
                return RunPowerShellScript(powershellExecutable, powershellArguments, hidePowerShellWindow);
            }
        }

        public string RunPowerShellScript(string powershellExecutable, string powershellArguments, bool hidePowerShellWindow = false)
        {
            string outputLog = ExecutePowerShellScript(powershellExecutable, powershellArguments, hidePowerShellWindow);
            CommandOutputLog = outputLog;
            return outputLog;
        }

        protected virtual string ExecutePowerShellScript(string powershellExecutable, string powershellArguments, bool hidePowerShellWindow = false)
        {
            var outputLog = new StringBuilder();
            try
            {
                var startInfo = new ProcessStartInfo()
                {
                    FileName = powershellExecutable,
                    Arguments = powershellArguments,
                    CreateNoWindow = hidePowerShellWindow,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                };
                startInfo.EnvironmentVariables["NO_COLOR"] = "1";
                using var process = Process.Start(startInfo) ?? throw new InvalidOperationException($"Failed to start {powershellExecutable}.");
                while (!process.StandardOutput.EndOfStream)
                {
                    outputLog.AppendLine(process.StandardOutput.ReadLine()); // Weirdly, PowerShell 7 won't give us new lines.
                }

                process.WaitForExit();
            }
            catch (Exception ex)
            {
                outputLog.Clear();
                outputLog.Append(ex);
            }

            return outputLog.ToString();
        }

        public async Task CheckCommandNotFoundRequirementsAsync()
        {
            if (_isRequirementsCheckRunning)
            {
                return;
            }

            _isRequirementsCheckRunning = true;
            IsCheckingRequirements = true;
            CommandOutputLog = string.Empty;
            try
            {
                CommandNotFoundRequirements requirements = await Task.Run(GetCommandNotFoundRequirements);

                isPowerShellPreviewDetected = requirements.IsPowerShellPreviewDetected;
                powerShellPreviewPath = requirements.PowerShellPreviewPath;
                CommandOutputLog = requirements.OutputLog;
                IsPowerShell7Detected = requirements.IsPowerShell7Detected;
                IsWinGetClientModuleDetected = requirements.IsWinGetClientModuleDetected;
                IsCommandNotFoundModuleInstalled = requirements.IsCommandNotFoundModuleInstalled;
                Logger.LogInfo(requirements.OutputLog);
            }
            finally
            {
                IsCheckingRequirements = false;
                _isRequirementsCheckRunning = false;
            }
        }

        private CommandNotFoundRequirements GetCommandNotFoundRequirements()
        {
            bool powerShell7Detected = false;
            bool powerShellPreviewDetected = false;
            string detectedPowerShellPreviewPath = null;
            var ps1File = AssemblyDirectory + "\\Assets\\Settings\\Scripts\\CheckCmdNotFoundRequirements.ps1";
            var arguments = $"-NoProfile -NonInteractive -ExecutionPolicy Unrestricted -File \"{ps1File}\"";
            var result = ExecutePowerShellScript("pwsh.exe", arguments, true);
            var outputLog = result;

            if (result.Contains("PowerShell 7.4 or greater detected."))
            {
                powerShell7Detected = true;
            }
            else if (result.Contains("PowerShell 7.4 or greater not detected."))
            {
                powerShell7Detected = false;
            }
            else if (result.Contains("pwsh.exe"))
            {
                // Likely an error saying there was an error starting pwsh.exe, so we can assume Powershell 7 was not detected.
                outputLog += "PowerShell 7.4 or greater not detected. Installation instructions can be found on https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-windows \r\n";
                powerShell7Detected = false;
            }

            if (!powerShell7Detected)
            {
                // powerShell Preview might be installed, check it.
                try
                {
                    // we have to search for the directory where the PowerShell preview command is located. It is added to the PATH environment variable, so we have to search for it there
                    foreach (string pathCandidate in Environment.GetEnvironmentVariable("PATH").Split(';'))
                    {
                        if (File.Exists(Path.Combine(pathCandidate, "pwsh-preview.cmd")))
                        {
                            result = ExecutePowerShellScript(Path.Combine(pathCandidate, "pwsh-preview.cmd"), arguments, true);
                            outputLog = result;
                            if (result.Contains("PowerShell 7.4 or greater detected."))
                            {
                                powerShellPreviewDetected = true;
                                powerShell7Detected = true;
                                detectedPowerShellPreviewPath = pathCandidate;
                                break;
                            }
                        }
                    }
                }
                catch (Exception)
                {
                    // nothing to do. No additional PowerShell installation found
                }
            }

            bool winGetClientModuleDetected = result.Contains("WinGet Client module detected.");
            bool commandNotFoundModuleInstalled = result.Contains("Command Not Found module is registered in the profile file.");

            return new CommandNotFoundRequirements(
                outputLog,
                powerShell7Detected,
                powerShellPreviewDetected,
                detectedPowerShellPreviewPath,
                winGetClientModuleDetected,
                commandNotFoundModuleInstalled);
        }

        public void InstallPowerShell7()
        {
            var ps1File = AssemblyDirectory + "\\Assets\\Settings\\Scripts\\InstallPowerShell7.ps1";
            var arguments = $"-NoProfile -ExecutionPolicy Unrestricted -File \"{ps1File}\"";
            var result = RunPowerShellOrPreviewScript("powershell.exe", arguments);
            if (result.Contains("Powershell 7 successfully installed."))
            {
                IsPowerShell7Detected = true;
            }

            Logger.LogInfo(result);

            // Update PATH environment variable to get pwsh.exe on further calls.
            Environment.SetEnvironmentVariable("PATH", (Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Machine) ?? string.Empty) + ";" + (Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.User) ?? string.Empty), EnvironmentVariableTarget.Process);
        }

        public void InstallWinGetClientModule()
        {
            var ps1File = AssemblyDirectory + "\\Assets\\Settings\\Scripts\\InstallWinGetClientModule.ps1";
            var arguments = $"-NoProfile -ExecutionPolicy Unrestricted -File \"{ps1File}\"";
            var result = RunPowerShellOrPreviewScript("pwsh.exe", arguments);
            if (result.Contains("WinGet Client module detected.") || result.Contains("WinGet Client module updated."))
            {
                IsWinGetClientModuleDetected = true;
            }
            else if (result.Contains("WinGet Client module not detected."))
            {
                IsWinGetClientModuleDetected = false;
            }

            Logger.LogInfo(result);
        }

        public void InstallModule()
        {
            var ps1File = AssemblyDirectory + "\\Assets\\Settings\\Scripts\\EnableModule.ps1";
            var arguments = $"-NoProfile -ExecutionPolicy Unrestricted -File \"{ps1File}\" -scriptPath \"{AssemblyDirectory}\\..\"";
            var result = RunPowerShellOrPreviewScript("pwsh.exe", arguments);

            if (result.Contains("Module is already registered in the profile file.")
                || result.Contains("Module was successfully registered in the profile file.")
                || result.Contains("Module was successfully upgraded in the profile file."))
            {
                IsCommandNotFoundModuleInstalled = true;
                PowerToysTelemetry.Log.WriteEvent(new CmdNotFoundInstallEvent());
            }

            Logger.LogInfo(result);
        }

        public void UninstallModule()
        {
            var ps1File = AssemblyDirectory + "\\Assets\\Settings\\Scripts\\DisableModule.ps1";
            var arguments = $"-NoProfile -ExecutionPolicy Unrestricted -File \"{ps1File}\"";
            var result = RunPowerShellOrPreviewScript("pwsh.exe", arguments);

            if (result.Contains("Removed the Command Not Found reference from the profile file.") || result.Contains("No instance of Command Not Found was found in the profile file."))
            {
                IsCommandNotFoundModuleInstalled = false;
                PowerToysTelemetry.Log.WriteEvent(new CmdNotFoundUninstallEvent());
            }

            Logger.LogInfo(result);
        }
    }
}
