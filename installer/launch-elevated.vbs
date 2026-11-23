' Launch sudocooler elevated without flashing a console window.
' ShellExecute with "runas" shows the UAC prompt directly. The show mode must
' be SW_SHOWNORMAL (1), not SW_HIDE (0): a GUI process started hidden keeps
' its window hidden, because the first ShowWindow call uses the process's
' STARTUPINFO show state. wscript.exe has no console of its own, so nothing
' needs hiding here.
Set shell = CreateObject("Shell.Application")
Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
app = fso.BuildPath(dir, "sudocooler.exe")
shell.ShellExecute app, "", dir, "runas", 1
