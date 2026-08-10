// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System.Collections.Generic;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;

namespace Microsoft.PowerToys.Settings.UI.Controls
{
    public sealed partial class QuickAccessList : UserControl
    {
        public QuickAccessList()
        {
            this.InitializeComponent();
        }

        public object ItemsSource
        {
            get => (object)GetValue(ItemsSourceProperty);
            set => SetValue(ItemsSourceProperty, value);
        }

        public static readonly DependencyProperty ItemsSourceProperty = DependencyProperty.Register(nameof(ItemsSource), typeof(object), typeof(QuickAccessList), new PropertyMetadata(null));

        private static void LoadDescription(object sender)
        {
            if (sender is FrameworkElement { DataContext: QuickAccessItem item })
            {
                item.LoadDescription();
            }
        }

        private void QuickAccessItem_GettingFocus(UIElement sender, GettingFocusEventArgs args)
        {
            LoadDescription(sender);
        }

        private void QuickAccessItem_PointerEntered(object sender, PointerRoutedEventArgs args)
        {
            LoadDescription(sender);
        }
    }
}
