# Kilim Windows（Rust 原生版）

这是 Kilim 的 Windows 移植版本，采用 Rust、Win32 和 Direct3D 11。当前打包版本为 0.2.0 功能对齐体验包，尚未完成全部实机验收。

2026-11-21：松手后立即开始淡出摩擦声，约 0.2 秒显著降低；全屏褶皱明暗在约 0.35 秒内减弱，再次抓取平滑恢复。63 项回归、GPU 导图和独立复审通过。新可执行文件为 `windows/target/release/kilim-windows-fade.exe`；请先从托盘退出当前版本，再运行新版，实际听感与持续拖拽效果待体验确认。

2026-11-21：针对全屏拖动时整层桌布闪退到桌面的现象，合并窗口位置、尺寸与像素提交，避免逐帧改变桌面子窗口层级。60 项 release 回归、clippy 及双屏桌面普通／全屏冒烟通过，已更新 `windows/target/release/kilim-windows.exe`，原录屏中的持续拖动待复测。

2026-11-20：修复前的本地改动已提交为 `56b94e1`。当前已修复全屏面片穿透、抓取片变尖／抓错层与高折面遮挡，完成 Raw Input、碰撞候选及 GPU 调度优化，并修复最新录屏中「鼠标到左上、折角仍卡右下」的碰撞死锁。59 项 release 回归、clippy 零警告、35 张离屏场景导出和独立代码复审通过；release 已重新构建，本地入口为 `windows/target/release/kilim-windows.exe`，现有 0.2.0 体验包尚未更新。新版原生持续拖拽手感与整机 CPU 满载仍待实机确认。

## 运行

先从旧版托盘菜单退出 Kilim，再解压新包并双击 `Kilim.exe`。程序显示一张桌面地毯，并在通知区域提供托盘菜单，无需管理员权限或联网。旧版仍运行时，新版会提示先退出旧版。

目标系统为 Windows 10／11 x64，需要 Explorer 桌面和支持 Direct3D 11 的显卡。当前仅在本机验证，其他系统版本、显卡、混合 DPI 环境待验证。

程序仅做视觉遮挡，不移动、不删除、不修改桌面文件，也不会修改桌面图标布局。

| 操作 | 效果 |
| --- | --- |
| 按住鼠标左键拖动地毯 | 抓取、移动、折叠 |
| 双击地毯 | 卷起／展开 |
| 按住 F8（默认，可在菜单修改） | 地毯渐变为半透明，鼠标穿透到桌面 |
| 拖动桌面图标 | 跟踪最近图标的凸起，松开后重新扫描真实坐标 |
| 右键地毯或托盘图标 | 显示菜单 |
| 菜单「重新铺平」 | 重置地毯位置和形状 |
| 菜单「全屏桌布」 | 用当前选中的图案铺满整个桌面；可直接抓起一角掀开，松手后自动垂回 |
| 菜单「退出 Kilim」 | 退出并移除地毯 |

透视期间如果已经按下鼠标，松开 F8 后仍保持穿透，直到释放鼠标。这样可以正常完成文件拖放。F8 避开了 Alt、Ctrl、Shift 对 Explorer 文件拖动行为的影响。

当前代码已加入完整 15 种预设、自定义图片、四档尺寸、自动／白天／傍晚／夜间光照、动态音效、21 种语言、偏好保存、可选透视键和用户主动开启的开机启动入口。

光照已恢复当时的昼夜明暗和色调处理，后来的原色保真修改已撤销；自定义照片仍沿用当时的 cover 重采样和织纹。

尺寸切换会保留现有位置和褶皱；正在拖拽或卷起时切换尺寸会重铺。隐藏状态下选择重新铺平会重新显示地毯。

图片导入可从花纹子菜单选择，也可把图片拖至托盘图标，出现临时接收面板后放下。Windows 托盘本身没有与 Mac 菜单栏相同的图片拖放接口，因此使用应用自己的面板承接，不改写 Explorer。

偏好和自定义照片保存在当前用户的 `%APPDATA%\Kilim`。开机启动默认关闭，仅点击对应菜单时创建或移除当前用户 Startup 文件夹内属于 Kilim 的快捷方式。

## 开发与构建

运行时代码使用 Microsoft 的 `windows`／`windows-core` SDK 封装，版本由 `Cargo.lock` 锁定。成品无需 Node.js、WebView 或其他图形引擎。图案资源的可复现构建工具使用 `scripts/package.json` 中的 Canvas2D，仅生成资源时需要，普通 Rust 构建直接使用已生成资源。

需要 Visual Studio C++ Build Tools、Windows SDK 和 Rust 1.88+ 的 MSVC x64 工具链。以下命令从仓库根目录执行。

已有 Rust 时直接运行：

```powershell
Push-Location windows
cargo build --release --locked
Pop-Location
```

如需项目内隔离安装 Rust，可使用以下脚本。它将官方安装器下载到 `.tools/` 并校验 SHA-256，将工具链和缓存也保存在 `.tools/`，不修改系统 PATH，不安装 Visual Studio 或 Windows SDK：

```powershell
.\windows\scripts\setup.ps1
.\windows\scripts\cargo.ps1 build --release --locked
```

静态 CRT 配置位于 `windows/.cargo/config.toml`。从 `windows/` 执行 Cargo 或使用项目脚本，确保该配置生效，使发行包无需额外安装 Visual C++ 运行库。

输出为 `windows/target/release/kilim-windows.exe`。正常使用无需保留 Rust 工具链。

## 验证

```powershell
.\windows\scripts\cargo.ps1 fmt --check
.\windows\scripts\cargo.ps1 test --locked
.\windows\scripts\cargo.ps1 -CargoArguments @('test', '--release', '--locked', '--', '--test-threads=1')
.\windows\scripts\cargo.ps1 -CargoArguments @('clippy', '--all-targets', '--locked', '--', '-D', 'warnings')
.\windows\scripts\cargo.ps1 -CargoArguments @('run', '--release', '--locked', '--', '--selftest', 'target/verification')
.\windows\scripts\cargo.ps1 -CargoArguments @('run', '--release', '--locked', '--', '--smoke-test')
```

- 当前 63 项 release 测试通过，覆盖物理、DPI、偏好、图片、菜单、本地化、连续碰撞、抓取保形与上层命中、GPU 遮挡及队列、鼠标按钮边沿、原生窗口提交及松手淡出。角落回归包含两种尺寸在 DPI 1.25 下从右下到左上的长拖、停住目标、逐帧跟随误差与无相交、松手落平；另有接触组静止 pin、共同移动与地面约束回归。旧优化版的原色色块及卷起过程验证不属于当前结果。
- `--selftest` 不创建桌面窗口，输出 35 张 PNG：20 张原有地毯场景，1920×1080 左拖、3440×1440 宽屏拖动，以及 2560×1600 从右下到左上长拖，各 4 张过程图与 1 张松手图。当前证据在 `windows/target/drag-follow-20261120/selftest/`；过程图已目视核对。普通地毯保留随机落下扰动，不要求跨次逐像素一致。上一轮 `drag-perf/` 的 10 张全屏图逐像素一致结论仅属于碰撞死锁修复之前。输出物理均耗时不包含渲染、桌面合成和输入，也不代表持续帧率。
- 本轮独立代码复审通过。全工程 `fmt --check` 仍有基线既存格式差异，本次新增／修改块已局部对齐，未整仓格式化。
- `--smoke-test` 会短暂创建真实桌面地毯和托盘，检查桌面父窗口、非置顶、透视命中穿透、菜单命令，并测量 30 帧计算／渲染／提交耗时，然后自动退出。
- 原有离屏诊断基准使用 `--selftest --bench`。新增 `--fullscreen-bench`，分别测量 2560×1600、4480×1600 下 120 步全屏拖动的物理和同步渲染回读，报告均值／P95／最大值；每步让出 4 毫秒，单帧超过 500 毫秒中止。可用 `--config-dir` 指定隔离图案配置，不创建桌面窗口、不写偏好。
- `--run-for-seconds 15` 会正常运行 15 秒后自动退出，可用于事件循环与资源开销验证。

生成本地便携包：

```powershell
.\windows\scripts\package.ps1
```

输出在根目录 `dist/`，包含可执行文件、说明、MIT 许可证以及 SHA-256 校验文件。此脚本只在本地打包，不上传或发布。

## 实现与限制

- `src/cloth.rs`、`src/collision.rs`：66×45 网格、Verlet/PBD 约束；全屏普通物理步骤使用顶点—面／边—边连续碰撞和安全限步，表面厚度为 2 个逻辑像素，普通地毯保留粒子自碰撞。全屏抓取按当前高度反投影并保留抓取片高度差；螺旋卷起流程沿用基线。
- 大折叠的残余接触使用临时碰撞组保留共同运动，避免局部碰撞将整张布反复限步到零。每次修正后重建候选并完整验证 CCD，最多 6 次，未解接触仍走安全限步；静止 pin 保持原位。松手后保留原厚度松弛，使折面能够落平。
- `src/desktop.rs`：发现 Explorer 桌面窗口，通过 Shell COM 的 `IFolderView` 只读获取图标位置。
- `src/renderer.rs`、`src/rug.hlsl`：Direct3D 11 渲染，WIC 解码仓库现有纹理。
- `src/app.rs`：桌面子窗口、分层透明合成、输入、托盘、固定步长循环及静止休眠。

透明显示采用 GPU 渲染后回读 BGRA，再通过 `UpdateLayeredWindow` 合成；透明像素由系统放行鼠标。透视时增加 `WS_EX_TRANSPARENT`。没有使用全局置顶窗口，也不注入 Explorer 或读取其进程内存。

鼠标输入使用 Raw Input 后台接收按钮边沿，移动在帧内采样；即时悬停命中仍保留。这样不再让系统鼠标事件同步等待绘制线程的全局低级钩子。帧调度避免高频输入饿死定时器或慢帧连续追算。GPU 三缓冲全忙时直接跳过提交、保留待绘状态，回读只复制最新已完成帧，阴影透明度为零时跳过阴影与柔化通道。相关机制依据 Microsoft 的 [LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc) 与 [Raw Input 示例](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input) 。

上一轮候选过滤的 900 步同轨迹哈希一致及约 27%～34% 提速证据保留在 `windows/target/drag-perf/`，不用于描述本轮接触处理改变后的形状。当前 `drag-follow-20261120/` 的中心拖动同步基准两种尺寸物理／渲染均值为 6.509／2.687 和 6.122／8.028 毫秒；新增角落大折叠物理均值为 21.524 毫秒，持续操作仍有优化空间。这些数据不包含 Explorer 合成和输入，不等于实际帧率或整机 CPU 降幅；静止后停止物理与 GPU 重绘，仅保留输入轮询及低频图标扫描。

当前使用每屏桌面子窗口、统一物理时钟，以及按主屏 DPI 换算的统一物理场。多显示器混合缩放、显示器热插拔、切换虚拟桌面、显示桌面操作及 Explorer 重启恢复，仍需进一步实机验收。

Explorer 暂不可用时会等待并尝试重建窗口；输入暂不可用时暂停抓取，恢复后继续。真实持续拖拽、F8 文件拖放、托盘图片拖入及音频设备行为尚未完成最终实机验收。2026-11-20 隔离预览曾一次以 `0xc0000409` 异常退出，之后 30 秒及 180 秒运行正常；本轮性能修复后 300 秒隔离预览退出码为 0，日志确认短抓取／松手与一次布面抬升。自动操作只持续几十毫秒，不能代替长时间拖动，也未复现用户报告的整机 CPU 满载；此前异常退出的原因仍待确认。

渲染已补入 4×MSAA、材质高光、低分辨率柔和阴影、mipmap、三缓冲及非阻塞 GPU 完成查询，并复用裁剪区域缓冲。完整验收状态见 `PARITY.md`。当前版本未签名、未公开发布。

上游项目及原版算法、纹理由 Yusuf Yılmaz 以 MIT 许可证提供，随包保留原始版权与许可声明。
