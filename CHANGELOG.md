## 0.10.0

* Fixing CI.
* Added simple backup functionality.

## 0.11.0

* Added `admin` parameter to `shell.exec()` function. and a while make_admin_command() in the util
* Also almost forgot to update the version smh

## 0.12.0

* Fixed a bug in the admin functionality...
* Switched to using PowerShell instead of cmd.

## 0.12.2

* Idk what im doing atp.

## 0.12.3

* Fixed a bug in the admin functionality...
* Switched to using PowerShell instead of cmd.

## 1.1.0

* Improved backup functionality.
* Backups no longer ignore hidden files.

## 1.2.0

* Added a new tool for reading the cheatsheet because rmcp doesn't support resources yet. And not to confuse the LLM.
* Overhauled the way Scripted shurikens start and stop by sharing the engine. Thus improving speed and efficiency.

## 1.3.0
* Optimized the shuriken discovery to improve performance by 15 ms.
* Switched the main directory to user's home directory.

## 1.4.0
* Fixed lua working directory errors

## 1.4.1
* Reworked the whole process spawning and shell exec functions in the lua stdlib

## 1.5.0
* Separated the proc module to include cwd.

## 1.6.0
* Added a custom file format for installing shurikens. For details on how it works, see the shuriken.hexpat file in the sdk folder.
* Added GUI support for installing shurikens.

## 1.7.0
* Switched the docs and cheatsheet from Resource to tools (read_cheatsheet, read_docs).
* Added the docs.

## 1.8.0 
* Added cwd to running scripts.