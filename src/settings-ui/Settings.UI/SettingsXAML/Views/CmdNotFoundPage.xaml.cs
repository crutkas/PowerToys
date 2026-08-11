// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using Microsoft.PowerToys.Settings.UI.Helpers;
using Microsoft.PowerToys.Settings.UI.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Microsoft.PowerToys.Settings.UI.Views
{
    public sealed partial class CmdNotFoundPage : NavigablePage
    {
        private CmdNotFoundViewModel ViewModel { get; set; }

        public CmdNotFoundPage()
        {
            ViewModel = new CmdNotFoundViewModel();
            DataContext = ViewModel;
            InitializeComponent();
            Loaded += CmdNotFoundPage_Loaded;
        }

        private async void CmdNotFoundPage_Loaded(object sender, RoutedEventArgs e)
        {
            Loaded -= CmdNotFoundPage_Loaded;
            await ViewModel.CheckCommandNotFoundRequirementsAsync();
        }

        private async void CheckCompatibility_Click(object sender, RoutedEventArgs e)
        {
            await ViewModel.CheckCommandNotFoundRequirementsAsync();
        }
    }
}
