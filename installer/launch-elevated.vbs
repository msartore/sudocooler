' Launch sudocooler elevated without flashing a console window.
' ShellExecute with "runas" shows the UAC prompt directly; the final 0 hides
' the launched window's console. wscript.exe has no console of its own.
Set shell = CreateObject("Shell.Application")
app = CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName) & "\sudocooler.exe"
shell.ShellExecute app, "", "", "runas", 0
