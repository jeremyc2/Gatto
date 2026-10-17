# Set up Gatto

[Overview](../README.md) · [Walkthrough](WALKTHROUGH.md) · [Support](../SUPPORT.md)

## 1. Install the app

1. [Download Gatto for Apple Silicon](https://github.com/jeremyc2/Gatto/releases/latest).
   Choose the file ending in `.dmg` under **Assets** on the release page.
2. Open the downloaded file to see the installation window.
3. Drag the Gatto icon onto the Applications folder shown below. Wait for the copy
   to finish.
4. Open Applications in Finder and double-click Gatto.

![Gatto's installation window: drag the cat app icon onto the Applications folder](images/dmg-install.png)

Install before enabling Start at Login so the saved app location is correct.
You can delete the downloaded file after installation. For macOS compatibility
and Intel Macs, see [Support](../SUPPORT.md#compatibility).

### Verify the download (optional)

The `.dmg` file is a disk image containing the app. Releases also include
`SHA256SUMS`. To verify the download, put both files in the same folder and run
this command in Terminal from that folder:

```sh
shasum -a 256 -c SHA256SUMS
```

## 2. Connect GitHub CLI

With [Homebrew](https://brew.sh/) installed:

```sh
brew install gh
gh auth login
gh auth status --hostname github.com
```

For other installation methods, see [GitHub CLI](https://cli.github.com/).
Choose **GitHub.com** and **HTTPS** during login. If a repository is missing,
see [repository troubleshooting](../SUPPORT.md#common-problems).

Gatto runs GitHub CLI as your signed-in macOS user. Standard Homebrew
installations work without extra configuration; see
[Support](../SUPPORT.md#common-problems) for a custom GitHub CLI location.

## 3. Choose your account or organization

On first launch, open **App Preferences** from the settings icon or the menu bar.
Gatto waits for an account or organization login before contacting GitHub.

<img src="images/walkthrough/01-first-run.png" alt="Gatto's first-run screen with Application Logs, App Preferences, walkthrough help, and an Open App Preferences button" width="620">

Enter a personal account login, such as `octocat`, or an organization login, such
as `acme`, in **GitHub account/organization**, and save it. Use the login rather
than a URL or `owner/repository`. Gatto lists repositories owned by that account
or organization that your active GitHub CLI account can access, including private
repositories when permitted. GitHub.com is the supported host.

Changing to a different account or organization clears the previous repository
selection and pins. Choose and pin repositories for the new owner.

Pin repositories you use often. Pins put them first in the repository picker
and enable the menu bar's clipboard actions.

<img src="images/walkthrough/02-preferences.png" alt="Gatto App Preferences with an account/organization, pinned repository, and global Quick Copy shortcut settings" width="620">

## 4. Set your preferences

- **Start at Login** opens Gatto at macOS login. If you move the app later, toggle
  this off and on to refresh its saved location.
- **Quick Copy shortcut** enables a global shortcut, initially
  **Command+Shift+G**. It is off by default and requires no Accessibility
  permission. See the [walkthrough](WALKTHROUGH.md#faster-clipboard-uploads) to
  record a different shortcut.
- **Close the window after copy** closes the main window after copying uploaded
  Markdown. Closing clears the attachment list, including uncopied results.

Ready to upload? Continue with the [walkthrough](WALKTHROUGH.md).
For reset, uninstall, or unreadable settings, see
[Support](../SUPPORT.md#reset-and-uninstall).
