// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System.Threading;
using System.Threading.Tasks;

using Microsoft.PowerToys.Settings.UI.ViewModels;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace ViewModelTests
{
    [TestClass]
    public class CmdNotFoundViewModelTest
    {
        private const string RequirementsOutput = """
            PowerShell 7.4 or greater detected.
            WinGet Client module detected.
            Command Not Found module is registered in the profile file.
            """;

        private sealed class TestCmdNotFoundViewModel : CmdNotFoundViewModel
        {
            private readonly ManualResetEventSlim _releaseProcess;
            private int _processExecutionCount;

            public TestCmdNotFoundViewModel(ManualResetEventSlim releaseProcess = null)
            {
                _releaseProcess = releaseProcess;
            }

            public int ProcessExecutionCount => Volatile.Read(ref _processExecutionCount);

            protected override string ExecutePowerShellScript(string powershellExecutable, string powershellArguments, bool hidePowerShellWindow = false)
            {
                Interlocked.Increment(ref _processExecutionCount);
                _releaseProcess?.Wait(5000);
                return RequirementsOutput;
            }
        }

        [TestMethod]
        public void ConstructorDoesNotRunRequirementsCheck()
        {
            var viewModel = new TestCmdNotFoundViewModel();

            Assert.AreEqual(0, viewModel.ProcessExecutionCount);
            Assert.IsTrue(viewModel.IsCheckingRequirements);
        }

        [TestMethod]
        public async Task RequirementsCheckRunsAsynchronouslyAndUpdatesState()
        {
            using var releaseProcess = new ManualResetEventSlim();
            var viewModel = new TestCmdNotFoundViewModel(releaseProcess);
            viewModel.CommandOutputLog = "stale output";

            Task checkTask = viewModel.CheckCommandNotFoundRequirementsAsync();
            Assert.IsTrue(SpinWait.SpinUntil(() => viewModel.ProcessExecutionCount == 1, 5000));
            Assert.IsFalse(checkTask.IsCompleted);
            Assert.AreEqual(string.Empty, viewModel.CommandOutputLog);

            releaseProcess.Set();
            await checkTask;

            Assert.IsFalse(viewModel.IsCheckingRequirements);
            Assert.IsTrue(viewModel.IsPowerShell7Detected);
            Assert.IsTrue(viewModel.IsWinGetClientModuleDetected);
            Assert.IsTrue(viewModel.IsCommandNotFoundModuleInstalled);
            Assert.AreEqual(RequirementsOutput, viewModel.CommandOutputLog);
        }
    }
}
