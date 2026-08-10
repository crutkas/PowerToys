// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using Microsoft.PowerToys.Settings.UI.Controls;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace ViewModelTests
{
    [TestClass]
    public class QuickAccessItemTests
    {
        [TestMethod]
        public void DescriptionIsLoadedOnDemandOnlyOnce()
        {
            var loadCount = 0;
            var item = new QuickAccessItem(() =>
            {
                loadCount++;
                return "Ctrl + Space";
            });

            Assert.AreEqual(0, loadCount);
            Assert.AreEqual(string.Empty, item.Description);
            Assert.IsTrue(item.HasDescription);

            item.LoadDescription();
            item.LoadDescription();

            Assert.AreEqual(1, loadCount);
            Assert.AreEqual("Ctrl + Space", item.Description);
            Assert.IsTrue(item.HasDescription);
        }

        [TestMethod]
        public void ItemWithoutDescriptionFactoryHasNoDescription()
        {
            var item = new QuickAccessItem();

            item.LoadDescription();

            Assert.AreEqual(string.Empty, item.Description);
            Assert.IsFalse(item.HasDescription);
        }

        [TestMethod]
        public void EmptyDescriptionHidesToolTipAfterLoad()
        {
            var item = new QuickAccessItem(() => string.Empty);

            Assert.IsTrue(item.HasDescription);

            item.LoadDescription();

            Assert.IsFalse(item.HasDescription);
        }
    }
}
