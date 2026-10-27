# 云存储 (rclone)

Xylem 没有内置专门的 Google Drive 或 S3 客户端，但也不需要：[rclone](https://rclone.org) 可以将它们（以及 OneDrive、Dropbox、Backblaze B2、Cloudflare R2、MinIO 等约 70 种其他服务）挂载为本地文件夹，Xylem 可以像操作普通文件夹一样操作它。所有功能均可正常使用：载入目录、Branch、标记、查看器、双向复制与移动、重命名、新建目录、删除。

## 为什么选择 rclone

我们选择配合 rclone 使用，而不是直接对接 Google Drive 和 Amazon S3 的接口。自己为每个服务编写代码只会增加出错的可能，而且直接接入 Google Drive 还会受到 Google 政策的限制：任何读取用户全部文件的应用都必须通过 Google 的验证和每年一次的安全评估，否则就会显示“未经验证的应用”警告并限制用户数量。至于 S3，其官方库体积庞大，会引入大量复杂性，而 Xylem 的设计理念正是尽可能避免不必要的复杂性。rclone 已经把这些工作做得非常完善，并且一次支持众多云服务。

## 初始设置（仅需一次）

安装 rclone（`apt install rclone`、`dnf install rclone`、`brew install rclone`，或从 [rclone.org](https://rclone.org/downloads/) 下载），然后运行 `rclone config` 创建一个远程配置 (remote)。您为它起的名字就是下面冒号前面的名称。

- **Google Drive：** 存储类型选择 `drive`；其余保持默认即可。rclone 会打开浏览器供您登录 Google 账号并授权。如果当前机器没有浏览器（如通过 SSH 连接），在浏览器问题上回答 *no*：rclone 会显示一条命令，供您在有浏览器的机器上运行 `rclone authorize`。
- **Amazon S3：** 存储类型选择 `s3`，提供商选择 `AWS`，然后可以输入访问密钥 (access key) 与秘密密钥 (secret)，或者选择 *env_auth* 以复用 AWS 工具已有的凭证（`~/.aws/credentials`、环境变量或实例角色）。其他兼容 S3 的服务（如 R2、MinIO、Wasabi 等）也在同一提供商列表中。

运行 `rclone lsd gdrive:`（或 `rclone lsd s3:`）可列出顶层文件夹（在 S3 中则是存储桶 bucket）：如果能正常列出，说明配置已就绪。

## 挂载与打开

在 Linux 上，假设远程配置名为 `gdrive`：

```bash
mkdir -p ~/gdrive
rclone mount gdrive: ~/gdrive --vfs-cache-mode writes --daemon
xy ~/gdrive
```

对于 S3，做法相同，使用 `s3:`（挂载所有存储桶）或 `s3:my-bucket`（仅挂载指定存储桶）。使用 `fusermount -u ~/gdrive` 卸载。

- `--vfs-cache-mode writes` 会在本地保留一份正在写入的文件副本，编辑器（`E`）和部分程序需要此模式才能正常工作。
- `--dir-cache-time 1h` 将目录列表缓存时间从默认的 5 分钟延长到 1 小时：重新载入（F3）会更快，但在别处所做的更改需要更长时间才会显示出来。

在 **macOS** 上，`rclone mount` 需要安装 [macFUSE](https://osxfuse.github.io)；而使用 `rclone nfsmount gdrive: ~/gdrive` 则无需安装任何额外组件。在 **Windows** 上，需要安装 [WinFsp](https://winfsp.dev) 并挂载为盘符：`rclone mount gdrive: X:`，然后运行 `xy X:\`。

为了方便随时访问，可以将挂载文件夹保存为 [书签](bookmarks.md)（Alt+B）。

在 Linux 上，Xylem 会自动识别由 rclone 挂载的文件夹：右侧 DISK 面板会显示云服务名称（如 Google Drive、Amazon S3、OneDrive 等），并在下方显示远程配置名（如 `rclone gdrive:`）。Info（Alt+I）和 Avail（`A`）也会显示这些信息。服务类型读取自 rclone 的配置文件（`~/.config/rclone/rclone.conf` 或 `$RCLONE_CONFIG`）；如果配置文件已加密，面板则只显示 rclone。

## 云端挂载的特殊之处

- **所有操作都经过网络。** 每次读取目录都是一次网络请求：在内容很多的 Drive 或存储桶上，按 `*`（载入整个分支）和 Branch（`B`）可能需要较长时间。建议只展开您需要访问的目录（按 → 或 `+`），因为目录是在打开时按需读取的，这样速度很快。
- **在别处所做的更改**（如在网页端或手机上）只有在缓存过期（`--dir-cache-time`）并按 F3 刷新后才会显示出来。
- **删除文件：** 请选择 `P`（永久删除）而不是 `T`（回收站）：在 Linux 上，挂载文件夹的回收站是位于其顶层的 `.Trash-` 文件夹（带有您的用户 ID），如果选 `T`，文件实际上仍会留在云端并继续占用云端空间。即使选择永久删除，Google Drive 也会把 rclone 删除的文件保留在云端自带的回收站里；而在 S3 上，除非存储桶开启了版本控制，否则删除的文件会立即彻底消失。
- **不保留权限与所有者：** 挂载点中的所有文件都显示相同的权限，且无法通过权限命令修改。修改日期会被保留。
- **Google 文档、表格和幻灯片** 并不是普通文件：rclone 会将它们显示为导出的格式（`.docx`、`.xlsx`、`.pptx`），且显示大小为 0。加上 `--drive-skip-gdocs` 参数可以隐藏它们。
- **S3 没有真正的目录**，只有包含 `/` 的对象名称：空目录在卸载后可能会消失，且重命名或移动目录实际上会逐个复制并删除其中的所有文件。

## 找出占满 Google Drive 空间的文件

当 Google Drive 不断提示您升级存储空间套餐时，在花钱扩容之前，您可以先用 Xylem 查明空间都去哪儿了。挂载 Drive，进入其根目录，按 `*` 载入全部内容（如果文件很多可能需要等一会儿），然后按 Alt+K → **Percent**（见 [目录大小](info.md)）：每个文件夹旁都会显示它占父文件夹的百分比，并以同级最大文件夹为基准绘制进度条。顺着最长的进度条一路找下去，就能揪出旧备份、大视频和重复文件。

[磁盘用量视图](usage.md)（在 Drive 根目录按 Alt+W）更进一步：将文件和文件夹混在一起按从大到小排列，因此无论是塞满文件的文件夹还是单个超大文件都无所遁形。此外，Branch（`B`）可以列出整个文件夹下的所有文件：按大小降序 [排序](sort.md)（按 Alt+S，再按 `O` 切换为降序，最后按 `S` 选择按大小）后，最大的文件就会排在最前面。

有几类内容会占用云端存储配额，但不会在挂载目录中显示出来：

- **云端硬盘的回收站** 在清空前（或满 30 天自动清除前）仍会占用空间。运行 `rclone about gdrive:` 可查看总容量、已用空间和回收站占用；挂载时加上 `--drive-trashed-only` 可以专门查看云端回收站里的内容。
- **Gmail 和 Google 相册** 与 Google Drive 共享同一存储配额，但挂载目录中只有云端硬盘里的文件。如果 `rclone about` 显示的已用空间远大于挂载目录统计出的总和，多出来的空间很可能用在了邮件和相册上。
- **Google 文档、表格和幻灯片** 虽然显示大小为 0，但较新创建的文档也会计入存储空间配额。
- **他人与您共享的文件** 不占用您的配额：它们计入文件所有者的存储空间。
