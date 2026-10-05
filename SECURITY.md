# Security policy

## Supported versions

Only the latest release receives security fixes. The app updates itself, so keeping automatic updates on is the best protection.

| Version        | Supported |
| -------------- | --------- |
| Latest release | Yes       |
| Older releases | No        |

## Reporting a vulnerability

Please do not open a public issue for security problems.

Report it privately through [GitHub private vulnerability reporting](https://github.com/chenoli/chenoli-auto-clicker/security/advisories/new). Include:

- what the problem is and what an attacker could do with it;
- steps to reproduce, or a proof of concept;
- the app version and your Windows version.

You can expect an acknowledgement within 7 days and an assessment within 14 days. Once a fix is released, the advisory is published with credit to the reporter, unless you prefer to stay anonymous.

## Scope

In scope:

- the CH. Clicker app and its installer;
- the update channel (`latest.json` and the update signature check);
- `.chprofile` import (files are validated by content and schema).

Out of scope:

- anti-cheat detection of the app: CH. Clicker never hides its activity by design;
- Windows SmartScreen or Smart App Control warnings on unsigned builds (see [`docs/release.md`](docs/release.md));
- problems that require an attacker who already controls your user account.
