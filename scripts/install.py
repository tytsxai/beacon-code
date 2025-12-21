#!/usr/bin/env python3
"""
Beacon Code 模块化安装脚本

支持选择性安装模块，JSON 配置驱动。

用法:
    python3 install.py                    # 安装默认模块
    python3 install.py --list-modules     # 列出可用模块
    python3 install.py --module cli sdk   # 安装指定模块
    python3 install.py --install-dir /opt/beacon  # 自定义安装目录
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
CONFIG_FILE = SCRIPT_DIR / "install.json"
LOG_FILE = SCRIPT_DIR / "install.log"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Beacon Code 模块化安装脚本",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--install-dir",
        type=Path,
        help="安装目录 (默认: ~/.beacon-code)",
    )
    parser.add_argument(
        "--module",
        dest="modules",
        action="append",
        help="要安装的模块 (可多次指定)",
    )
    parser.add_argument(
        "--list-modules",
        action="store_true",
        help="列出所有可用模块",
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=CONFIG_FILE,
        help="配置文件路径 (默认: scripts/install.json)",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="强制覆盖已存在的文件",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="仅显示将执行的操作，不实际执行",
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="显示详细输出",
    )
    return parser.parse_args()


def load_config(config_path: Path) -> dict[str, Any]:
    """加载 JSON 配置文件"""
    if not config_path.exists():
        raise FileNotFoundError(f"配置文件不存在: {config_path}")
    with open(config_path, "r", encoding="utf-8") as f:
        return json.load(f)


def expand_path(path_str: str, install_dir: Path) -> Path:
    """展开路径中的变量"""
    expanded = path_str.replace("${INSTALL_DIR}", str(install_dir))
    expanded = os.path.expanduser(expanded)
    return Path(expanded)


def log(msg: str, verbose: bool = False) -> None:
    """输出日志"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_line = f"[{timestamp}] {msg}"
    if verbose:
        print(log_line)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(log_line + "\n")


def list_modules(config: dict[str, Any]) -> None:
    """列出所有可用模块"""
    modules = config.get("modules", {})
    default_modules = config.get("default_modules", [])

    print("可用模块:")
    print("-" * 50)
    for name, info in modules.items():
        is_default = "✓" if name in default_modules else " "
        desc = info.get("description", "无描述")
        ops_count = len(info.get("operations", []))
        print(f"  [{is_default}] {name:12} - {desc} ({ops_count} 个操作)")
    print("-" * 50)
    print(f"默认模块: {', '.join(default_modules)}")


def op_copy_file(
    src: str,
    dest: str,
    install_dir: Path,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """复制单个文件"""
    src_path = REPO_ROOT / src
    dest_path = expand_path(dest, install_dir)

    if not src_path.exists():
        log(f"源文件不存在: {src_path}", verbose)
        return False

    if dest_path.exists() and not force:
        log(f"目标已存在 (使用 --force 覆盖): {dest_path}", verbose)
        return False

    if dry_run:
        print(f"  [DRY-RUN] 复制: {src_path} -> {dest_path}")
        return True

    dest_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src_path, dest_path)
    log(f"复制: {src_path} -> {dest_path}", verbose)
    return True


def op_copy_dir(
    src: str,
    dest: str,
    install_dir: Path,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """复制目录"""
    src_path = REPO_ROOT / src
    dest_path = expand_path(dest, install_dir)

    if not src_path.exists():
        log(f"源目录不存在: {src_path}", verbose)
        return False

    if dest_path.exists():
        if not force:
            log(f"目标目录已存在 (使用 --force 覆盖): {dest_path}", verbose)
            return False
        if not dry_run:
            shutil.rmtree(dest_path)

    if dry_run:
        print(f"  [DRY-RUN] 复制目录: {src_path} -> {dest_path}")
        return True

    shutil.copytree(src_path, dest_path)
    log(f"复制目录: {src_path} -> {dest_path}", verbose)
    return True


def op_run_command(
    command: str,
    install_dir: Path,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """执行命令"""
    expanded_cmd = command.replace("${INSTALL_DIR}", str(install_dir))

    if dry_run:
        print(f"  [DRY-RUN] 执行: {expanded_cmd}")
        return True

    log(f"执行: {expanded_cmd}", verbose)
    try:
        result = subprocess.run(
            expanded_cmd,
            shell=True,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            log(f"命令失败: {result.stderr}", verbose)
            return False
        return True
    except Exception as e:
        log(f"命令异常: {e}", verbose)
        return False


def op_merge_json(
    src: str,
    dest: str,
    install_dir: Path,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """合并 JSON 文件"""
    src_path = REPO_ROOT / src
    dest_path = expand_path(dest, install_dir)

    if not src_path.exists():
        log(f"源 JSON 不存在: {src_path}", verbose)
        return False

    with open(src_path, "r", encoding="utf-8") as f:
        src_data = json.load(f)

    if dest_path.exists():
        with open(dest_path, "r", encoding="utf-8") as f:
            dest_data = json.load(f)
        # 深度合并
        merged = {**dest_data, **src_data}
    else:
        merged = src_data

    if dry_run:
        print(f"  [DRY-RUN] 合并 JSON: {src_path} -> {dest_path}")
        return True

    dest_path.parent.mkdir(parents=True, exist_ok=True)
    with open(dest_path, "w", encoding="utf-8") as f:
        json.dump(merged, f, indent=2, ensure_ascii=False)
        f.write("\n")

    log(f"合并 JSON: {src_path} -> {dest_path}", verbose)
    return True


def execute_operation(
    op: dict[str, Any],
    install_dir: Path,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """执行单个操作"""
    op_type = op.get("type")
    desc = op.get("description", "")

    if verbose or dry_run:
        print(f"  {desc}")

    if op_type == "copy_file":
        return op_copy_file(
            op["src"], op["dest"], install_dir, force, dry_run, verbose
        )
    elif op_type == "copy_dir":
        return op_copy_dir(
            op["src"], op["dest"], install_dir, force, dry_run, verbose
        )
    elif op_type == "run_command":
        return op_run_command(op["command"], install_dir, dry_run, verbose)
    elif op_type == "merge_json":
        return op_merge_json(
            op["src"], op["dest"], install_dir, force, dry_run, verbose
        )
    else:
        log(f"未知操作类型: {op_type}", verbose)
        return False


def install_module(
    name: str,
    module_config: dict[str, Any],
    install_dir: Path,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> bool:
    """安装单个模块"""
    desc = module_config.get("description", name)
    operations = module_config.get("operations", [])

    print(f"\n安装模块: {name} - {desc}")
    print("-" * 40)

    success_count = 0
    for op in operations:
        if execute_operation(op, install_dir, force, dry_run, verbose):
            success_count += 1

    total = len(operations)
    if success_count == total:
        print(f"✓ 模块 {name} 安装完成 ({success_count}/{total})")
        return True
    else:
        print(f"✗ 模块 {name} 部分失败 ({success_count}/{total})")
        return False


def main() -> int:
    args = parse_args()

    try:
        config = load_config(args.config)
    except FileNotFoundError as e:
        print(f"错误: {e}", file=sys.stderr)
        return 1
    except json.JSONDecodeError as e:
        print(f"配置文件格式错误: {e}", file=sys.stderr)
        return 1

    if args.list_modules:
        list_modules(config)
        return 0

    # 确定安装目录
    if args.install_dir:
        install_dir = args.install_dir.expanduser().resolve()
    else:
        default_dir = config.get("install_dir", "~/.beacon-code")
        install_dir = Path(os.path.expanduser(default_dir)).resolve()

    # 确定要安装的模块
    modules_config = config.get("modules", {})
    if args.modules:
        selected_modules = args.modules
    else:
        selected_modules = config.get("default_modules", [])

    # 验证模块存在
    for mod in selected_modules:
        if mod not in modules_config:
            print(f"错误: 未知模块 '{mod}'", file=sys.stderr)
            print(f"可用模块: {', '.join(modules_config.keys())}")
            return 1

    print(f"Beacon Code 安装器 v{config.get('version', '1.0')}")
    print(f"安装目录: {install_dir}")
    print(f"选中模块: {', '.join(selected_modules)}")

    if args.dry_run:
        print("\n[DRY-RUN 模式 - 不会实际执行操作]")

    # 创建安装目录
    if not args.dry_run:
        install_dir.mkdir(parents=True, exist_ok=True)

    # 安装模块
    results = []
    for mod_name in selected_modules:
        success = install_module(
            mod_name,
            modules_config[mod_name],
            install_dir,
            args.force,
            args.dry_run,
            args.verbose,
        )
        results.append((mod_name, success))

    # 汇总
    print("\n" + "=" * 50)
    success_count = sum(1 for _, s in results if s)
    total = len(results)

    if success_count == total:
        print(f"✓ 安装完成: {success_count}/{total} 模块成功")
        return 0
    else:
        print(f"✗ 安装部分完成: {success_count}/{total} 模块成功")
        return 1


if __name__ == "__main__":
    sys.exit(main())
