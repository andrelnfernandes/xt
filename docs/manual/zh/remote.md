# 远程主机 (SSH)

Xylem 可以通过 SSH 将另一台机器上的文件作为目录树显示出来：既可以单独显示，也可以在 [分屏](split.md) 模式下与本机并排显示。

## 打开远程连接

在 `L`（[Log](treespec.md)）中输入地址，或者在命令行启动 `xy` 时直接指定：

- `user@host:` — 主目录；`user@host:/srv/www` — 指定目录；
- `ssh://[user@]host[:port][/path]` — 指定端口，或省略用户名（使用 `~/.ssh/config` 中的配置）。

Xylem 直接调用系统自带的 `ssh`，因此您在终端中能用的配置在这里同样有效：`~/.ssh/config`（主机别名、端口、ProxyJump）、密钥、ssh-agent 以及 `known_hosts`。无需在远程机器上安装任何额外软件：Xylem 使用所有 SSH 服务器都自带的 SFTP 协议通信（支持 Linux、BSD、macOS、Windows）。

## 密码与验证提示

当 ssh 需要询问信息时，提示会显示在底部状态栏中：

- **密码** 或密钥的 **通行短语 (passphrase)**：输入时不回显；按 Enter 发送，按 Esc 放弃连接。Xylem 只负责将其传递给 ssh，不会保存任何密码。
- **未知主机 (unknown host)**：显示主机名及其密钥指纹，供您在输入 `yes` 前核对（确认后 ssh 会像往常一样将其记录到 `known_hosts` 中）。

连接过程中按 `Esc` 可取消。如果配置了密钥和 ssh-agent，则无需任何手动输入。

## 支持的功能

目录树、文件窗口、[Filespec](filespec.md)、Branch、Showall、标记、[查看器](viewer.md)、[Ctrl+S 与 Ctrl+V](search.md)、自动预览 (Autoview)、Info（Alt+I）、各种显示模式（所有者模式显示 SFTP 返回的用户 ID 和组 ID）、[比较](compare.md) 以及打开（`O` 会下载副本并在本机打开）。

**复制 (Copy)** 可将远程文件下载到本机，并保留其权限和修改日期（精确到秒，与 SFTP 一致）；在分屏状态下，会默认建议将本机窗口作为目标目录。Alt+C（保留路径）同样可用，路径从远程窗口的根目录算起。

目前远程端为 **只读** 模式：移动、删除、重命名、新建目录、权限属性、修改日期、编辑、执行命令以及将文件复制 *到* 远程主机暂不支持。按 F3 重新读取目录；远程目录不会自动刷新。

在远程窗口中按 **F8** 会打开一个本机窗口（位于启动 Xylem 时的文件夹），方便将远程与本地并排操作。

## 远程主机上的终端

在远程窗口中按 **Alt+X** 会在该远程机器上打开一个 shell（`ssh -t`），初始位于当前窗口所在的远程目录；在本地窗口中按 Alt+X 则像以前一样打开本地 shell（切换窗口时，两个 shell 都会在后台保持运行）。在 Linux 和 macOS 上，远程 shell 会复用已建立的 SSH 连接，因此无需再次输入密码；在 Windows 上，ssh 会在终端内提示输入密码。

当远程 shell 通过 OSC 7 序列上报其当前目录时，目录树会自动跟随远程 shell 的 `cd` 移动。许多系统默认已配置好此功能；对于 bash，可在 **远程主机上** 的 `~/.bashrc` 中添加以下内容：

```bash
PROMPT_COMMAND='printf "\033]7;file://%s%s\007" "$HOSTNAME" "$PWD"'
```

Xylem 绝不会向远程 shell 自动输入命令（因为它无法判断远程终端里是否正在运行其他程序），而且 Alt+O 和 `$XY_TAGGED` 仅适用于本地终端。当远程终端打开时退出 Xylem 会先询问确认。

## 登录时自动启动的 tmux 或 screen

有些用户会在远程主机的 `~/.bashrc` 中配置 SSH 登录时自动启动 tmux（或 screen），这样即使连接中断（如 Wi-Fi 不稳定、VPN 重连）也不会终止正在运行的程序，重新登录即可恢复同一会话。例如：

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

如果这样配置，Xylem 的远程终端也会在 tmux 内部打开。除了 **目录树无法跟随 shell 的 `cd`** 之外，其余功能均正常：因为 tmux 位于 shell 和 Xylem 之间，它会拦截 OSC 7 序列而不向外传递。

Xylem 启动远程 shell 时也会像本地终端一样设置 `XY_TERMINAL=1`，以便登录脚本区分这两种情况。在判断条件中加入对该变量的检查，即可在普通 `ssh` 登录时继续使用 tmux，而在 Xylem 连接时不启动 tmux：

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ] && [ -z "$XY_TERMINAL" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

在 Xylem 内部，shell 不需要靠 tmux 来保活：只要 Xylem 打开着，它就会一直运行；而如果网络连接断开，远程窗口本身也会随之断开。

关于 Google Drive、S3 等云端存储，请参阅 [云存储](cloud.md)。
