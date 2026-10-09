# Gatto walkthrough

Gatto turns a local image into GitHub-ready Markdown. This walkthrough covers the normal setup and upload flow.

## 1. Set up the app

The first-run screen points you to App Preferences. Gatto will not contact GitHub until an organization has been configured.

<img src="images/walkthrough/01-first-run.png" alt="Gatto's first-run screen with an Open App Preferences button" width="620">

Open **App Preferences**, enter the GitHub organization whose repositories you want to use, and save it. Gatto uses the active GitHub CLI account, so that account must be able to access the organization.

## 2. Choose a repository

Select the repository that should own the uploaded attachment. Pin repositories you use regularly to keep them at the top of the picker and enable the menu bar's clipboard shortcuts.

<img src="images/walkthrough/02-preferences.png" alt="Gatto App Preferences with an organization and pinned repository configured" width="620">

Settings are stored locally. GitHub authentication remains managed by GitHub CLI.

## 3. Stage an image

Return to the main window and paste an image, drag an image into the drop zone, or click the drop zone to select a file. PNG, JPEG, GIF, and WebP images are supported.

<img src="images/walkthrough/03-image-staged.png" alt="Gatto with its rocket cat artwork staged and ready to upload" width="620">

The attachment stays in Gatto while it uploads. You can edit its description, remove it, or preview it before sending.

## 4. Upload and copy the Markdown

Click **Upload image**. After GitHub accepts the attachment, its status changes to **Uploaded** and the attachment gains a copy button.

<img src="images/walkthrough/04-upload-complete.png" alt="Gatto showing an uploaded image with a Markdown copy action" width="620">

Copy the generated Markdown and paste it into a GitHub issue, pull request, discussion, or Markdown file. The result has this form:

```md
![Gatto rocket cat](https://github.com/user-attachments/assets/…)
```

## Faster clipboard uploads

After pinning a repository, copy an image and use one of these actions from Gatto's menu bar icon:

- **Preview from Clipboard** stages the image in the main window.
- **Quick Copy** uploads it in the background, copies its URL, and sends a macOS notification when it finishes.

## Updating these screenshots

From the repository root, run:

```sh
./scripts/update-walkthrough-screenshots.sh
```

The script renders the production GPUI interface through its off-screen Metal renderer using fixed, in-memory example data. It does not read or modify Gatto preferences, contact GitHub, use UI automation, or require Screen Recording permission. The command is macOS-only and overwrites the PNG files under `docs/images/walkthrough/`.
