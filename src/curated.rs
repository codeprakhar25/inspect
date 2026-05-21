//! Built-in knowledge base for common commands.

use crate::sources::{CommandDocs, Example, Flag, RiskLevel, RiskNote};

#[allow(clippy::too_many_lines)]
pub fn fetch(cmd: &str) -> Option<CommandDocs> {
    let name = cmd.rsplit('/').next().unwrap_or(cmd);
    let docs = match name {
        // ── File operations ───────────────────────────────────────────────────
        "cp" => entry(
            "copy files and directories",
            "cp [OPTION]... SOURCE DEST",
            &[
                ("-r, -R, --recursive", "copy directories recursively"),
                ("-a, --archive", "preserve mode, ownership, timestamps, links, and other metadata"),
                ("-i, --interactive", "prompt before overwriting an existing destination"),
                ("-n, --no-clobber", "do not overwrite an existing destination"),
                ("-v, --verbose", "print each copy operation"),
            ],
            &[
                ("cp file.txt backup/file.txt", "copy one file to another path"),
                ("cp -r src/ backup/src/", "copy a directory recursively"),
                ("cp -a project/ project-backup/", "copy a directory while preserving metadata"),
                ("cp -n *.txt archive/", "copy files without overwriting existing names"),
            ],
            &[
                (RiskLevel::Caution, "Overwrites existing files by default unless you use -i or -n."),
                (RiskLevel::Info, "Shell globs like *.txt are expanded before cp receives the arguments."),
            ],
        ),
        "mv" => entry(
            "move or rename files and directories",
            "mv [OPTION]... SOURCE DEST",
            &[
                ("-i, --interactive", "prompt before overwriting an existing destination"),
                ("-n, --no-clobber", "do not overwrite an existing destination"),
                ("-v, --verbose", "print each move operation"),
                ("-t, --target-directory", "move all source arguments into a directory"),
            ],
            &[
                ("mv old.txt new.txt", "rename a file"),
                ("mv file.txt notes/", "move a file into a directory"),
                ("mv -i *.log archive/", "move matching files and prompt before overwrites"),
            ],
            &[
                (RiskLevel::Caution, "Can overwrite destination files unless protected with -i or -n."),
                (RiskLevel::Caution, "Moving across filesystems copies then removes the source."),
            ],
        ),
        "rm" => entry(
            "remove files or directories",
            "rm [OPTION]... FILE...",
            &[
                ("-i", "prompt before every removal"),
                ("-I", "prompt once before large or recursive removals"),
                ("-r, -R, --recursive", "remove directories and their contents recursively"),
                ("-f, --force", "ignore missing files and never prompt"),
                ("-v, --verbose", "print each removal"),
            ],
            &[
                ("rm file.txt", "remove one file"),
                ("rm -i *.log", "prompt before removing matching files"),
                ("rm -r old-dir/", "remove a directory tree"),
            ],
            &[
                (RiskLevel::Danger, "Destructive command: removed files are normally not recoverable."),
                (RiskLevel::Danger, "Be especially careful with -r, -f, shell globs, and variables."),
            ],
        ),
        "ls" => entry(
            "list directory contents",
            "ls [OPTION]... [FILE]...",
            &[
                ("-l", "use long listing format"),
                ("-a, --all", "show entries starting with ."),
                ("-h, --human-readable", "show sizes in human-readable units with -l"),
                ("-t", "sort by modification time"),
                ("-R, --recursive", "list subdirectories recursively"),
            ],
            &[
                ("ls", "list the current directory"),
                ("ls -lah", "show all files with details and readable sizes"),
                ("ls -lt", "show newest entries first"),
            ],
            &[],
        ),
        "find" => entry(
            "search for files in a directory hierarchy",
            "find [PATH...] [EXPRESSION]",
            &[
                ("-name PATTERN", "match file names with a shell-style pattern"),
                ("-type f|d", "match files or directories"),
                ("-maxdepth N", "limit recursion depth"),
                ("-mtime N", "match by modification time in days"),
                ("-exec CMD {} \\;", "run a command for each match"),
            ],
            &[
                ("find . -name '*.rs'", "find Rust files below the current directory"),
                ("find logs -type f -mtime +30", "find log files older than 30 days"),
                ("find . -type f -name '*.tmp' -delete", "delete matching temp files"),
            ],
            &[
                (RiskLevel::Caution, "-delete and -exec can affect many files; test without them first."),
                (RiskLevel::Info, "Quote patterns like '*.rs' so the shell does not expand them early."),
            ],
        ),
        "grep" => entry(
            "search text using patterns",
            "grep [OPTION]... PATTERNS [FILE]...",
            &[
                ("-r, --recursive", "search directories recursively"),
                ("-n, --line-number", "print line numbers"),
                ("-i, --ignore-case", "ignore case distinctions"),
                ("-v, --invert-match", "select non-matching lines"),
                ("-E, --extended-regexp", "use extended regular expressions"),
            ],
            &[
                ("grep -n 'TODO' src/*.rs", "find TODO lines with line numbers"),
                ("grep -R \"api_key\" .", "search recursively under the current directory"),
                ("grep -vi error app.log", "show lines that do not match error, case-insensitively"),
            ],
            &[
                (RiskLevel::Info, "Quote patterns to avoid shell expansion or escaping surprises."),
            ],
        ),
        "tar" => entry(
            "archive files",
            "tar [OPTION...] [FILE]...",
            &[
                ("-c, --create", "create a new archive"),
                ("-x, --extract", "extract files from an archive"),
                ("-f, --file", "use archive file name"),
                ("-z, --gzip", "filter archive through gzip"),
                ("-v, --verbose", "list files processed"),
            ],
            &[
                ("tar -czf project.tar.gz project/", "create a gzipped archive"),
                ("tar -xzf project.tar.gz", "extract a gzipped archive"),
                ("tar -tf project.tar.gz", "list archive contents"),
            ],
            &[
                (RiskLevel::Caution, "Extracting archives can overwrite files in the destination."),
                (RiskLevel::Info, "List with -t before extracting archives from untrusted sources."),
            ],
        ),
        "ps" => entry(
            "report process status",
            "ps [OPTION]...",
            &[
                ("aux", "BSD-style listing for all processes"),
                ("-ef", "System V-style full listing"),
                ("-p PID", "show a specific process"),
                ("-o FORMAT", "choose output columns"),
            ],
            &[
                ("ps aux", "show all processes in BSD format"),
                ("ps -ef | grep nginx", "search a full process listing"),
                ("ps -p 1234 -o pid,ppid,cmd", "show selected columns for one PID"),
            ],
            &[],
        ),
        "kill" => entry(
            "send a signal to a process",
            "kill [-SIGNAL] PID...",
            &[
                ("-TERM, -15", "ask a process to terminate"),
                ("-KILL, -9", "force a process to stop immediately"),
                ("-HUP, -1", "send hangup, often used to reload daemons"),
                ("-l", "list signal names"),
            ],
            &[
                ("kill 1234", "send SIGTERM to process 1234"),
                ("kill -HUP 1234", "ask a daemon to reload if it supports SIGHUP"),
                ("kill -9 1234", "force-kill a stuck process"),
            ],
            &[
                (RiskLevel::Caution, "Signals affect running processes immediately."),
                (RiskLevel::Danger, "kill -9 prevents normal cleanup; use it only when gentler signals fail."),
            ],
        ),
        "curl" => entry(
            "transfer data from or to a URL",
            "curl [OPTION...] URL",
            &[
                ("-L, --location", "follow redirects"),
                ("-I, --head", "fetch headers only"),
                ("-o, --output", "write output to a file"),
                ("-X, --request", "set the HTTP method"),
                ("-H, --header", "send a custom header"),
            ],
            &[
                ("curl -I https://example.com", "show response headers"),
                ("curl -L -o file.zip https://example.com/file.zip", "download a URL following redirects"),
                ("curl -X POST -H 'Content-Type: application/json' -d '{}' URL", "send a JSON POST request"),
            ],
            &[
                (RiskLevel::Caution, "Piping curl output directly into a shell executes remote code."),
                (RiskLevel::Info, "Use -f with scripts if HTTP errors should fail the command."),
            ],
        ),
        "ssh" => entry(
            "OpenSSH remote login client",
            "ssh [OPTION]... [user@]host [COMMAND]",
            &[
                ("-i IDENTITY_FILE", "select a private key file"),
                ("-p PORT", "connect to a specific port"),
                ("-L", "set up local port forwarding"),
                ("-N", "do not execute a remote command"),
                ("-v", "print verbose debug output"),
            ],
            &[
                ("ssh user@example.com", "open an interactive remote shell"),
                ("ssh -i key.pem user@example.com", "connect with a specific key"),
                ("ssh -L 8080:localhost:80 user@example.com", "forward local port 8080 to the remote host"),
            ],
            &[
                (RiskLevel::Caution, "Remote commands run on the target host, not your local machine."),
                (RiskLevel::Info, "Check the host key warning before accepting a new remote identity."),
            ],
        ),
        "cd" => entry(
            "change the shell working directory",
            "cd [DIRECTORY]",
            &[],
            &[
                ("cd project/", "enter a directory"),
                ("cd ..", "move to the parent directory"),
                ("cd -", "return to the previous directory"),
            ],
            &[
                (RiskLevel::Info, "cd is a shell builtin; external programs cannot change your parent shell directory."),
            ],
        ),

        // ── Git ───────────────────────────────────────────────────────────────
        "git" => entry(
            "track changes and collaborate on code",
            "git <command> [options]",
            &[
                ("--version", "print the Git version"),
                ("--help", "show help"),
                ("-C <path>", "run as if started in the given path"),
                ("--no-pager", "do not pipe output through a pager"),
            ],
            &[
                ("git status", "show working tree status"),
                ("git log --oneline", "compact one-line commit history"),
                ("git diff", "show unstaged changes"),
            ],
            &[
                (RiskLevel::Info, "git never modifies remote history without an explicit push or force-push"),
            ],
        ),
        "git-commit" => entry(
            "record staged changes as a new commit",
            "git commit [options]",
            &[
                ("-m <msg>", "use the given message as the commit message"),
                ("-a", "automatically stage all tracked modified and deleted files"),
                ("--amend", "replace the tip of the current branch with a new commit"),
                ("-v", "show a unified diff of what would be committed in the editor"),
                ("--no-edit", "reuse the previous commit message without opening an editor"),
            ],
            &[
                ("git commit -m \"fix typo\"", "commit staged changes with a message"),
                ("git commit -am \"update readme\"", "stage tracked files and commit"),
                ("git commit --amend --no-edit", "add staged changes to the last commit silently"),
            ],
            &[
                (RiskLevel::Caution, "--amend rewrites history; don't amend commits already pushed to a shared branch"),
            ],
        ),
        "git-push" => entry(
            "upload local commits to a remote repository",
            "git push [remote] [branch]",
            &[
                ("-u, --set-upstream", "set the upstream tracking branch"),
                ("--force", "overwrite the remote branch — destructive"),
                ("--force-with-lease", "force push only if nobody else has pushed"),
                ("--tags", "push all tags"),
                ("--dry-run", "show what would be pushed without doing it"),
            ],
            &[
                ("git push", "push current branch to its tracked remote"),
                ("git push -u origin main", "push and set upstream for the main branch"),
                ("git push --force-with-lease", "safely force-push after a rebase"),
            ],
            &[
                (RiskLevel::Danger, "--force overwrites the remote branch; use --force-with-lease instead to avoid overwriting others' work"),
            ],
        ),
        "git-pull" => entry(
            "fetch and merge changes from the remote",
            "git pull [remote] [branch]",
            &[
                ("--rebase", "rebase local commits on top of fetched commits instead of merging"),
                ("--no-commit", "fetch and merge but do not create a merge commit"),
                ("--ff-only", "only fast-forward; abort if a merge commit would be needed"),
                ("--autostash", "stash local changes before pulling and re-apply after"),
            ],
            &[
                ("git pull", "fetch and merge from the tracked remote"),
                ("git pull --rebase origin main", "rebase onto remote main instead of merging"),
            ],
            &[
                (RiskLevel::Info, "git pull --rebase keeps a cleaner history than the default merge strategy"),
            ],
        ),
        "git-log" => entry(
            "show commit history",
            "git log [options] [revision range] [[--] path]",
            &[
                ("--oneline", "compact one-commit-per-line output"),
                ("--graph", "draw an ASCII branch graph"),
                ("--all", "show commits from all branches and tags"),
                ("-n <number>", "limit output to this many commits"),
                ("--author=<pattern>", "filter commits by author"),
                ("--since=<date>", "show commits newer than a date"),
            ],
            &[
                ("git log --oneline -10", "show the last 10 commits, one per line"),
                ("git log --graph --all --oneline", "visual branch history for all refs"),
                ("git log --author=\"Alice\" --since=\"1 week ago\"", "recent commits by Alice"),
            ],
            &[],
        ),
        "git-diff" => entry(
            "show changes between commits, working tree, or index",
            "git diff [options] [commit] [--] [path]",
            &[
                ("--staged, --cached", "show changes staged for the next commit vs HEAD"),
                ("--stat", "show a diffstat summary instead of a full patch"),
                ("--word-diff", "show changes at the word level"),
                ("-w", "ignore all whitespace when comparing lines"),
            ],
            &[
                ("git diff", "show unstaged changes in the working tree"),
                ("git diff --staged", "show what is staged and ready to commit"),
                ("git diff HEAD~1", "show changes introduced by the last commit"),
                ("git diff --stat main", "summarise changes between HEAD and main"),
            ],
            &[],
        ),

        // ── Permissions ───────────────────────────────────────────────────────
        "chmod" => entry(
            "change file permissions",
            "chmod [OPTION]... MODE FILE...",
            &[
                ("-R", "apply the permission change recursively to directories"),
                ("-v", "output a diagnostic for every file processed"),
                ("--reference=FILE", "copy permissions from the given reference file"),
            ],
            &[
                ("chmod +x script.sh", "make a script executable"),
                ("chmod 644 file.txt", "set read/write for owner, read-only for others"),
                ("chmod -R 755 dir/", "set standard directory permissions recursively"),
                ("chmod u+x,g-w file", "add execute for owner, remove write for group"),
            ],
            &[
                (RiskLevel::Danger, "chmod -R 777 gives everyone full access — almost always wrong"),
                (RiskLevel::Caution, "Removing execute on a directory prevents entering it"),
            ],
        ),
        "chown" => entry(
            "change file owner and group",
            "chown [OPTION]... [OWNER][:[GROUP]] FILE...",
            &[
                ("-R", "change ownership recursively"),
                ("-v", "output a diagnostic for every file processed"),
                ("--reference=FILE", "copy owner:group from the given reference file"),
            ],
            &[
                ("chown alice file.txt", "give alice ownership of a file"),
                ("chown alice:staff dir/", "set owner and group together"),
                ("chown -R www-data /var/www", "give the web server user ownership recursively"),
            ],
            &[
                (RiskLevel::Caution, "Requires root or sudo for most system files"),
                (RiskLevel::Danger, "chown -R on the wrong directory can break system services"),
            ],
        ),

        // ── Text processing ───────────────────────────────────────────────────
        "sed" => entry(
            "stream editor — filter and transform text",
            "sed [OPTION]... 'SCRIPT' [FILE]...",
            &[
                ("-i", "edit files in place (modifies the original file)"),
                ("-n", "suppress default output; only print with explicit p command"),
                ("-e <script>", "add an expression to the script"),
                ("-r, -E", "use extended regular expressions"),
            ],
            &[
                ("sed 's/foo/bar/g' file.txt", "replace all occurrences of foo with bar"),
                ("sed -i 's/old/new/g' file.txt", "edit the file in place"),
                ("sed -n '10,20p' file.txt", "print only lines 10 through 20"),
                ("sed '/^#/d' file.txt", "delete comment lines starting with #"),
            ],
            &[
                (RiskLevel::Caution, "sed -i modifies files in place with no undo — back up first or use -i.bak"),
                (RiskLevel::Info, "Without -i, sed writes to stdout and does not touch the file"),
            ],
        ),
        "awk" => entry(
            "process and transform structured text line by line",
            "awk [OPTION]... 'PROGRAM' [FILE]...",
            &[
                ("-F <sep>", "set the input field separator"),
                ("-v var=val", "assign a value to a variable before running"),
                ("-f <file>", "read the awk program from a file"),
            ],
            &[
                ("awk '{print $1}' file.txt", "print the first field of each line"),
                ("awk -F':' '{print $1}' /etc/passwd", "print usernames from passwd"),
                ("awk '$3 > 100 {print $0}' data.txt", "print lines where field 3 exceeds 100"),
                ("awk '{sum += $1} END {print sum}' numbers.txt", "sum the first column"),
            ],
            &[
                (RiskLevel::Info, "$0 is the whole line; $1, $2... are whitespace-separated fields by default"),
            ],
        ),
        "cat" => entry(
            "concatenate files and print to standard output",
            "cat [OPTION]... [FILE]...",
            &[
                ("-n", "number all output lines"),
                ("-A", "show non-printing characters, tabs, and line endings"),
                ("-s", "squeeze multiple adjacent blank lines into one"),
                ("-v", "show non-printing characters using caret notation"),
            ],
            &[
                ("cat file.txt", "print a file to the terminal"),
                ("cat file1.txt file2.txt > combined.txt", "concatenate two files into one"),
                ("cat -n script.sh", "print a script with line numbers"),
            ],
            &[],
        ),
        "head" => entry(
            "output the first part of a file",
            "head [OPTION]... [FILE]...",
            &[
                ("-n N", "print the first N lines instead of the default 10"),
                ("-c N", "print the first N bytes"),
            ],
            &[
                ("head -20 file.txt", "show the first 20 lines"),
                ("head -1 file.csv", "show only the header row of a CSV"),
            ],
            &[],
        ),
        "tail" => entry(
            "output the last part of a file",
            "tail [OPTION]... [FILE]...",
            &[
                ("-n N", "print the last N lines instead of the default 10"),
                ("-f", "follow the file and print new lines as they arrive"),
                ("-c N", "print the last N bytes"),
            ],
            &[
                ("tail -f /var/log/syslog", "watch a log file in real time"),
                ("tail -100 app.log", "show the last 100 lines of a log"),
            ],
            &[
                (RiskLevel::Info, "tail -f stays open and prints new lines as they arrive — useful for live logs"),
            ],
        ),
        "sort" => entry(
            "sort lines of text",
            "sort [OPTION]... [FILE]...",
            &[
                ("-n", "compare fields as numbers, not strings"),
                ("-r", "reverse the sort order"),
                ("-k N", "sort by the Nth whitespace-delimited field"),
                ("-u", "output only the first of repeated equal lines"),
                ("-t SEP", "use SEP as the field delimiter"),
            ],
            &[
                ("sort file.txt", "sort lines alphabetically"),
                ("sort -n numbers.txt", "sort numerically"),
                ("sort -t: -k3 -n /etc/passwd", "sort password file by UID"),
                ("sort -u list.txt", "sort and remove duplicate lines"),
            ],
            &[],
        ),
        "uniq" => entry(
            "filter or count adjacent duplicate lines",
            "uniq [OPTION]... [INPUT [OUTPUT]]",
            &[
                ("-c", "prefix each output line with its occurrence count"),
                ("-d", "only print lines that appear more than once"),
                ("-u", "only print lines that appear exactly once"),
                ("-i", "ignore case when comparing lines"),
            ],
            &[
                ("sort file.txt | uniq", "remove duplicate lines after sorting"),
                ("sort file.txt | uniq -c | sort -rn", "count and rank duplicates"),
                ("uniq -d sorted.txt", "show only the lines that repeat"),
            ],
            &[
                (RiskLevel::Info, "uniq only removes adjacent duplicates — always sort first unless you want positional dedup"),
            ],
        ),
        "wc" => entry(
            "count lines, words, and characters",
            "wc [OPTION]... [FILE]...",
            &[
                ("-l", "print the line count only"),
                ("-w", "print the word count only"),
                ("-c", "print the byte count only"),
                ("-m", "print the character count only"),
            ],
            &[
                ("wc -l file.txt", "count lines in a file"),
                ("wc -w essay.txt", "count words in a file"),
                ("ls | wc -l", "count the number of items in a directory listing"),
            ],
            &[],
        ),
        "diff" => entry(
            "compare files line by line",
            "diff [OPTION]... FILE1 FILE2",
            &[
                ("-u", "output a unified diff with 3 lines of context"),
                ("-r", "compare directories recursively"),
                ("--color", "highlight added and removed lines with colour"),
                ("-i", "ignore case differences"),
                ("-w", "ignore all whitespace when comparing"),
            ],
            &[
                ("diff file1.txt file2.txt", "compare two files"),
                ("diff -u original.txt modified.txt", "show a patch-style unified diff"),
                ("diff -r dir1/ dir2/", "compare two directory trees"),
            ],
            &[],
        ),
        "rsync" => entry(
            "fast, incremental file transfer and sync",
            "rsync [OPTION]... SOURCE DEST",
            &[
                ("-a, --archive", "archive mode: preserve permissions, timestamps, symlinks, owner, group"),
                ("-v", "increase verbosity"),
                ("-z", "compress data during the transfer"),
                ("--progress", "show per-file transfer progress"),
                ("--delete", "remove files in DEST that are not present in SOURCE"),
                ("-n, --dry-run", "perform a trial run without making any changes"),
                ("--exclude=PATTERN", "exclude files matching PATTERN from the sync"),
            ],
            &[
                ("rsync -av src/ dest/", "sync a local directory verbosely"),
                ("rsync -avz user@host:src/ dest/", "sync from a remote host with compression"),
                ("rsync -av --delete src/ dest/", "mirror src/ to dest/, removing extra files"),
                ("rsync -avn src/ dest/", "dry-run to preview what would change"),
            ],
            &[
                (RiskLevel::Danger, "--delete removes files in DEST that don't exist in SOURCE — always dry-run first with -n"),
                (RiskLevel::Info, "Trailing slash on source means 'copy contents of', no slash means 'copy the directory itself'"),
            ],
        ),

        // ── Network ───────────────────────────────────────────────────────────
        "ping" => entry(
            "check network connectivity to a host",
            "ping [OPTION]... HOST",
            &[
                ("-c N", "stop after sending N packets"),
                ("-i SEC", "wait SEC seconds between packets"),
                ("-W SEC", "time to wait for a response per packet"),
            ],
            &[
                ("ping google.com", "check connectivity to google.com"),
                ("ping -c 4 8.8.8.8", "send exactly 4 pings to Google DNS"),
                ("ping -c 1 -W 1 192.168.1.1", "quick single-ping check of a local host"),
            ],
            &[],
        ),

        // ── System info ───────────────────────────────────────────────────────
        "which" => entry(
            "find the full path of a shell command",
            "which COMMAND",
            &[],
            &[
                ("which python3", "find the python3 binary in PATH"),
                ("which node", "check where node is installed"),
            ],
            &[],
        ),
        "whereis" => entry(
            "locate binary, source and man page files for a command",
            "whereis COMMAND",
            &[],
            &[
                ("whereis python3", "show all locations for python3"),
            ],
            &[],
        ),

        // ── Docker ────────────────────────────────────────────────────────────
        "docker" => entry(
            "build and run containerized applications",
            "docker <command> [options]",
            &[
                ("--help", "show help for a docker command"),
                ("--version", "print Docker version information"),
                ("-D", "enable debug mode"),
            ],
            &[
                ("docker ps", "list running containers"),
                ("docker images", "list locally available images"),
                ("docker logs <container>", "fetch the logs of a container"),
                ("docker exec -it <container> bash", "open a shell inside a running container"),
            ],
            &[
                (RiskLevel::Info, "Containers run as root by default inside unless the image specifies a user"),
                (RiskLevel::Caution, "docker run without --rm leaves stopped containers on disk"),
            ],
        ),
        "docker-run" => entry(
            "create and start a new container from an image",
            "docker run [OPTIONS] IMAGE [COMMAND]",
            &[
                ("-d", "run the container in the background (detached mode)"),
                ("-p host:container", "publish a container port to the host"),
                ("-v host:container", "mount a host directory into the container"),
                ("--rm", "automatically remove the container when it exits"),
                ("-e KEY=VAL", "set an environment variable inside the container"),
                ("--name <name>", "assign a name to the container"),
                ("-it", "allocate an interactive terminal"),
            ],
            &[
                ("docker run --rm alpine echo hello", "run a one-shot command in alpine"),
                ("docker run -d -p 8080:80 nginx", "start nginx in the background on port 8080"),
                ("docker run -it --rm ubuntu bash", "start an interactive Ubuntu shell"),
            ],
            &[
                (RiskLevel::Caution, "Without --rm the container stays on disk after stopping — use docker ps -a to see them"),
            ],
        ),

        // ── Kubernetes ────────────────────────────────────────────────────────
        "kubectl" => entry(
            "control Kubernetes clusters",
            "kubectl <command> [flags]",
            &[
                ("-n, --namespace", "target a specific Kubernetes namespace"),
                ("--context", "use a named context from kubeconfig"),
                ("--kubeconfig", "path to the kubeconfig file"),
                ("-o", "output format: yaml, json, wide, name, etc."),
                ("--dry-run=client", "preview changes without applying them"),
            ],
            &[
                ("kubectl get pods", "list pods in the current namespace"),
                ("kubectl describe pod <name>", "show detailed info about a pod"),
                ("kubectl logs <pod> -f", "stream logs from a pod"),
                ("kubectl apply -f manifest.yaml", "apply a configuration file to the cluster"),
            ],
            &[
                (RiskLevel::Info, "Always check your current context with: kubectl config current-context"),
                (RiskLevel::Caution, "Commands run against whichever cluster your kubeconfig context points to"),
            ],
        ),

        _ => return None,
    };

    Some(docs)
}

fn entry(
    summary: &str,
    usage: &str,
    flags: &[(&str, &str)],
    examples: &[(&str, &str)],
    risks: &[(RiskLevel, &str)],
) -> CommandDocs {
    CommandDocs {
        summary: Some(summary.to_string()),
        usage: Some(usage.to_string()),
        flags: flags
            .iter()
            .map(|(names, description)| Flag {
                names: (*names).to_string(),
                description: (*description).to_string(),
            })
            .collect(),
        examples: examples
            .iter()
            .map(|(command, description)| Example {
                command: (*command).to_string(),
                description: Some((*description).to_string()),
            })
            .collect(),
        risks: risks
            .iter()
            .map(|(level, message)| RiskNote {
                level: level.clone(),
                message: (*message).to_string(),
            })
            .collect(),
        sources: vec!["curated".to_string()],
    }
}
