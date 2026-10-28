# Privacy

Knobset collects nothing. It has no accounts, analytics, crash reporting,
ads or telemetry, and it never connects to the internet.

## What stays on your device

- **Backups** of your pager's profile, including its serial number,
  zones, knobs and talkgroups. On a Mac, Linux or Windows they're in the
  app's data folder or the folder you pick in Settings; on an iPad they're
  in Knobset's folder in the Files app.
- **Unication's templates**, imported from your own copy of their PPS
  installer.
- **Settings**: those folders and the pager's address.

Knobset doesn't send any of this anywhere. On an iPad, deleting the app
removes all of it. On a Mac, Linux or Windows, delete the app's data folder
too, and any backups folder you picked yourself in Settings: Knobset never
deletes a folder you chose.

## The only connection

Knobset talks to one address: the pager itself, at `10.10.0.12`, over the
pager's USB cable. That's why the iPad asks for Local Network access. The
pager's access password, if it has one, is checked with the pager and
never stored, logged or shown by Knobset. The pager's own protocol sends
it unencrypted over the USB cable, as it does with Unication's software,
so use a password you don't use anywhere else.

## Others who handle your data

- **Apple's TestFlight** (iPad beta): if you install the beta, Apple
  shares with the developer the feedback and screenshots you choose to
  send, crash reports, and your device model and iPadOS version, under
  [Apple's privacy policy](https://www.apple.com/legal/privacy/).
- **GitHub** hosts the downloads, issues and discussions, under
  [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
  Anything you post in an issue is public.
- **Email** to hello@knobset.com is forwarded by Cloudflare to the
  maintainer's inbox and isn't shared with anyone else.

## Questions

Write to hello@knobset.com. If this policy changes, the change is in this
file's history.
