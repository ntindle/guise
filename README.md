<p align="center">
  <img src="assets/logo.png" alt="Guise Logo" width="140">
</p>

<h1 align="center">Guise</h1>

<p align="center">Run all your Claude Desktop accounts at once — each in its own window, each staying logged in. No logging out, no re-entering your password.</p>

---

Got a work account and a personal one? Normally Claude Desktop only holds **one**
login at a time — to use the other you log out, retype your email, wait for the
code, log back in, and the first account is now logged out. Every time.

**guise** gives each account its own private Claude profile, so they all stay
logged in side by side. You just open the one you want.

```console
$ guise work        # opens (or focuses) your work account's window
$ guise personal    # personal's window — already logged in, no reload
```

## How it's different

guise doesn't copy or swap your session around (that fails — Claude ties a login
to one profile and drops it the moment you sign out). Instead, each account lives
in its **own permanent Claude profile folder**, launched independently. Logging
into one account never touches another, so nothing ever gets logged out.

- **Everything stays on your machine.** guise never reads, copies, or decrypts
  your login — Claude owns each folder; guise only decides which one to open.
- **Accounts stay live together.** Open two (or more) Claude windows at once and
  click between them.
- **Set up once.** Log into each account a single time; it stays logged in.

## Install

**Quick install** (macOS):

```console
curl -fsSL https://raw.githubusercontent.com/siddhjagani/guise/main/install.sh | bash
```

**Quick install** (Windows, in PowerShell):

```powershell
irm https://raw.githubusercontent.com/siddhjagani/guise/main/install.ps1 | iex
```

**Homebrew** (macOS):

```console
brew install siddhjagani/tap/guise
```

**From source** (needs a Rust toolchain):

```console
git clone https://github.com/siddhjagani/guise
cd guise && ./install.sh        # macOS
cd guise; ./install.ps1         # Windows (PowerShell)
```

macOS (Apple Silicon + Intel) and Windows (x86_64) are supported. Run
`guise doctor` any time to check things are in order.

## Get started

**1. Save your first account.** This opens a fresh Claude window:

```console
$ guise add work
✓ Created work. A fresh Claude window is opening — log into this account there.
```

Log into that window normally. Done — work stays logged in from now on.

**2. Save your other account.** Opens a *second* window (the first stays logged in):

```console
$ guise add personal
✓ Created personal. A fresh Claude window is opening — log into this account there.
```

Log into that one too.

**3. From now on, just open whichever you want:**

```console
$ guise work        # opens work (or brings its window to the front)
$ guise personal    # opens personal
```

Both windows can be open at the same time — switch by clicking, instantly, with
no reload and no login screen.

## Everyday commands

```console
guise               # pick an account from a menu and open it
guise <name>        # open that account (● = already open)
guise all           # open every account's window at once
guise add <name>    # save a new account (opens a window to log into)
guise ls            # list your accounts (● marks the ones open now)
guise rm <name>     # forget an account (closes its window, deletes its profile)
guise doctor        # check everything's set up
```

Running `guise` on its own gives you a menu (with an "open all" option on top):

```console
$ guise
? Open which account?
❯ ★ Open ALL accounts
  1. work ●  ·  you@company.com
  2. personal  ·  you@gmail.com
```

Add `--email you@company.com` to `guise add` if you want your email shown in the
list (Claude doesn't expose it, so guise can't fill it in for you):

```console
guise add work --email you@company.com
```

## See your accounts

```console
$ guise ls
● 1. work        ·  you@company.com
  2. personal    ·  you@gmail.com
```

(The ● means that account's window is open right now.)

## Telling the windows apart

Every window is still named **"Claude"** — the app can't be renamed without
breaking Anthropic's code signature (it then refuses to launch), so guise can't
give each window a custom title. To move between open accounts:

- **`guise all`** brings them all up, then use **Mission Control** (macOS, swipe
  up / F3) or **Task View** (Windows, Win+Tab) to see every window at once.
- **Cmd-`** (backtick) on macOS, or **Alt+Tab** / **Ctrl+Win+arrows** on Windows,
  cycles through the windows.
- Each window's title reflects its own account's workspace/chat, so they're not
  identical on screen.

## Is everything okay?

```console
$ guise doctor
guise checkup

  ✓ Claude.app               /Applications/Claude.app
  ✓ Claude executable        /Applications/Claude.app/Contents/MacOS/Claude
  ✓ guise store              /Users/you/.config/guise

  Saved accounts:
    1. work  ·  you@company.com ● running
    2. personal  ·  you@gmail.com

  overall: healthy ✓
```

(On Windows the app paths look like
`C:\Users\you\AppData\Local\Programs\Claude` for the direct install, or the
Microsoft Store's `claude-desktop` alias — guise finds either automatically.)

## Where guise keeps its files

Each account is a folder under `~/.config/guise/accounts/` (`%USERPROFILE%\.config\guise\accounts\`
on Windows) — its own complete Claude profile. Delete that folder and guise
forgets every account (each one's login just goes away with its folder; nothing
else on your machine is touched).

> **Upgrading?** If you have an existing `~/.guise` directory, guise automatically
> migrates it to `~/.config/guise` on first run and leaves a compatibility symlink
> at the old path. You can also set `$XDG_CONFIG_HOME` to override the location.

## Settings

```console
guise config                                    # show settings
guise config set app-path /path/Claude.app      # macOS, if Claude.app moved
guise config set app-path C:\path\to\Claude     # Windows, if auto-detect misses
```

Add `--json` to `guise ls` or `guise doctor` for scriptable output.

## Works with meld

If you also use meld to merge Claude Code chats across accounts, guise points
every account's `claude-code-sessions` at one shared folder so meld can see
them all. This is on by default and inert if you don't use meld:

```console
guise config set share-code-sessions off   # opt out
guise config set code-sessions-root <dir>  # override the shared folder
```

By default the shared folder is meld's configured `sessions_root`, falling back
to Claude's standard location. On Windows the link is a directory symlink, which
needs **Developer Mode** (Settings > System > For developers) or an elevated
shell — without it, guise prints a note and keeps going.

## Good to know

- macOS (Apple Silicon + Intel) and Windows (x86_64) are supported.
- **Each account stays on this machine** — the profiles are tied to this machine
  and aren't meant to be copied elsewhere.
- Running two Claude windows uses roughly twice the memory of one — expected,
  since they're two real app instances.

## Uninstall

```console
rm "$(which guise)"      # remove the program (macOS)
rm -rf ~/.config/guise   # remove your saved accounts (optional)
rm -f ~/.guise           # remove the compatibility symlink, if present
```

```powershell
Remove-Item "$env:LOCALAPPDATA\guise\bin\guise.exe"  # remove the program (Windows)
Remove-Item -Recurse "$env:USERPROFILE\.config\guise"  # saved accounts (optional)
```

## License

MIT
