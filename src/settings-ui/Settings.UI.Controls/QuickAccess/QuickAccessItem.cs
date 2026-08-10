// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System;
using System.Windows.Input;
using Microsoft.PowerToys.Settings.UI.Library.Helpers;
using Microsoft.UI.Xaml;

namespace Microsoft.PowerToys.Settings.UI.Controls
{
    public sealed class QuickAccessItem : Observable
    {
        private Func<string>? _descriptionFactory;

        public QuickAccessItem()
        {
        }

        public QuickAccessItem(Func<string>? descriptionFactory)
        {
            _descriptionFactory = descriptionFactory;
        }

        private string _title = string.Empty;

        public string Title
        {
            get => _title;
            set => Set(ref _title, value);
        }

        private string _description = string.Empty;

        public string Description
        {
            get => _description;
            set => Set(ref _description, value);
        }

        public bool HasDescription => _descriptionFactory is not null || !string.IsNullOrEmpty(_description);

        public void LoadDescription()
        {
            var descriptionFactory = _descriptionFactory;
            if (descriptionFactory is null)
            {
                return;
            }

            var description = descriptionFactory();
            _descriptionFactory = null;
            Description = description;
            OnPropertyChanged(nameof(HasDescription));
        }

        private string _icon = string.Empty;

        public string Icon
        {
            get => _icon;
            set => Set(ref _icon, value);
        }

        private ICommand? _command;

        public ICommand? Command
        {
            get => _command;
            set => Set(ref _command, value);
        }

        private object? _commandParameter;

        public object? CommandParameter
        {
            get => _commandParameter;
            set => Set(ref _commandParameter, value);
        }

        private bool _visible = true;

        public bool Visible
        {
            get => _visible;
            set => Set(ref _visible, value);
        }

        private object? _tag;

        public object? Tag
        {
            get => _tag;
            set => Set(ref _tag, value);
        }
    }
}
