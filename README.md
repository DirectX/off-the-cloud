# Off the Cloud

**Off the Cloud** is a command-line tool written in Rust for migrating mailboxes from one IMAP server to another. The tool uses a two-stage migration process consisting of the `pull` and `push` subcommands of the `imap` command to securely transfer mailbox data, maintaining folder structure and message integrity. 

This tool is ideal for migrating email between servers or backing up email to a local storage structure.

## Features

- [x] **Two-Stage Migration**: Migrate mailboxes in two steps with `pull` (download) and `push` (upload) commands.
- [x] **Configurable Storage**: Save messages to a specified directory with user-specific folders and a mirrored IMAP folder structure.
- [x] **File-Based Storage**: Emails are saved as individual files in `.eml` format, e.g., `00000001.eml`.
- [x] **Incremental Pulling**: Only new messages are downloaded in repeated `pull` actions.
- [x] **Customizable IMAP Folder Structure**: Set custom folder name mappings for localization and modify the delimiter for folder hierarchy.

## Upcoming Features
- [ ] **Calendar Backups**: CalDAV, *.ical support.
- [ ] **WebDAV transfers**: WebDAV support.

## Installation

To install `Off the Cloud`, you will need Rust installed. Once Rust is set up, run:

```bash
git clone https://github.com/DirectX/off-the-cloud.git
cd off-the-cloud
cargo install --path ./
```

Copy example configuration and change settings to necessary for source and target server configs. Optional values are commented.

```
cp config.example.yaml config.yaml
cp .example.env .env
```

## Usage

Run `off-the-cloud --help` to see general usage instructions.

### IMAP Migration Workflow

1. **Pull**: Download emails from the source IMAP server.
2. **Push**: Upload emails to the target IMAP server.

### Example Workflow

1. **Configure** `config.yaml` to specify IMAP server settings for the `pull` and `push` stages. See [Configuration](#configuration).
2. **Pull** emails from the source server:

   ```bash
   off-the-cloud imap pull --email user@example.com --password <PASSWORD> --out-dir messages
   ```

3. **Push** emails to the destination server:

   ```bash
   off-the-cloud imap push --email new_user@example.com --password <PASSWORD> --in-dir messages
   ```

### CLI Commands

The application has the following command structure:

```text
off-the-cloud <COMMAND> [OPTIONS]
```

#### `imap pull`

Downloads messages from the source IMAP server and stores them locally as `.00000001.eml`.

**Options:**
- `--email`: Email address for the source account.
- `--password`: Password for the source account.
- `--out-dir`: Output directory for stored messages (default: `messages`).
- `--export-mbox`: Optionally export messages in Mbox format for further importing manually. No `*.eml` files storing in this mode and `imap push` wouldn't work after.
- `--max-file-size`: File size limit for Mbox exports (only if `--export-mbox` is set).

Re-creates email mailbox structure inside of the `{out_dir}` folder and placing messages in the following format: `.00000001.eml` in folders respective to IMAP folder structure e.g.

```
./messages
└── example.com
    ├── user1@example.com
    │   ├── INBOX
    │   │   ├── News
    │   │   │   ├── .00000001.eml
    │   │   │   ├── .00000002.eml
    │   │   │   └── ...
    │   │   ├── .00000001.eml
    │   │   ├── .00000002.eml
    │   │   ├── .00000003.eml
    │   │   └── ...
    │   └── Sent
    │       ├── .00000001.eml
    │       ├── .00000002.eml
    │       └── ...
    └── user2@example.com
        ├── INBOX
        │   ├── News
        │   │   ├── .00000001.eml
        │   │   └── ...
          ...
```

> [!NOTE]
> Command `imap pull` is resumable. It is safe to run it repeatedly. The process will continue for every folder from the latest file.

#### `imap push`

**Options:**
- `--email`: Email address for the destination account.
- `--password`: Password for the destination account.
- `--in-dir`: Input directory containing downloaded messages (default: `messages`).

> [!NOTE]
> Call `imap push` can be called more than once. Repetitive call of `imap push` command will upload messages not uploaded yet.

Uploads messages to the destination IMAP server. It will traverse the internal structure of given mailbox and re-creates IMAP folders if necessary. Only dot-prefixed messages like `.00000001.eml` will be processed. Upon successfull upload the file name `.00000001.eml` will be changed to `00000001.eml` in order to exclude it from further uploads.

The contents of individual mailbox can be archived for backup purposes as follows

```bash
DOMAIN=example.com
MAILBOX=user1@example.com
tar -cvzf ./messages/${DOMAIN}/$(date +%Y%m%d_%H%M%S)-${MAILBOX}-backup.tar.gz ./messages/${DOMAIN}/${MAILBOX}
```

> [!TIP]
> For backing-up all mailboxes in domain use [./scripts/backup-domain.sh](#backup-domain).

## Configuration

**Off the Cloud** uses a configuration file `config.yaml` to specify IMAP server details, custom folder delimiters, and folder name mappings. Example:

```yaml
imap:
  pull:
    server: imap.gmail.com
    port: 993
  push:
    server: imap.example.com
    port: 993
    folder_delimiter: /
    folder_name_mappings:
      "Envoyés": "Sent"
      "Corbeille": "Trash"
      "Pourriel": "Junk"
```

### Configuration Options

- **server**: IMAP server address.
- **port**: Port for IMAP connections (e.g., 993 for SSL).
- **folder_delimiter**: Character for folder hierarchy (e.g., `.` or `/`).
- **folder_name_mappings**: Mappings for IMAP folder names, allowing localized folder names to be translated (e.g., French to English).

## Convenient Scripts

### Pull

```bash
./scripts/pull.sh user1@example.com password1
```

### Pull CSV

```bash
./scripts/pull-csv.sh ./messages/pull-list.csv
```

> [!NOTE]
> Comma-separated CSV file containing 2 columns `email` and `password` without header.
> Use file `sample-pull-list.csv` as a reference.

### Push

```bash
./scripts/push.sh user1@example.com anotherPassword1
```

### Push CSV

```bash
./scripts/push-csv.sh ./messages/push-list.csv
```

> [!NOTE]
> Comma-separated CSV file containing 2 columns `email` and `password` without header.
> Use file `sample-push-list.csv` as a reference.

### Backup Domain

```bash
./scripts/backup-domain.sh example.com
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contribution

Contributions, issues, and feature requests are welcome!