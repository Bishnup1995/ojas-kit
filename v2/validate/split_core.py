"""Known-good l3 solution: move _check_iter, Parameter, Option, Argument from core.py into params.py.
argv[1] = 'good' (re-export from core) or 'bad' (no re-export)."""
import ast, sys
p = "src/click/core.py"; src = open(p).read().splitlines(keepends=True)
start = next(i for i, l in enumerate(src) if l.startswith("def _check_iter("))
end = next(i for i, l in enumerate(src) if l.startswith("def __getattr__("))
header_end = next(i for i, l in enumerate(src) if l.startswith("_HELP_OPTION_STORAGE_NAME") or l.startswith("F = "))
moved = src[start:end]; rest = src[:start] + src[end:]
tree = ast.parse("".join(rest))
names = [n.name for n in tree.body if isinstance(n, (ast.FunctionDef, ast.ClassDef)) and n.name != "__getattr__"]
names += [t.id for n in tree.body if isinstance(n, ast.Assign) for t in n.targets if isinstance(t, ast.Name)]
hdr = "".join(src[:header_end])
params = hdr + "\nfrom .core import " + ", ".join(sorted(set(names))) + "  # noqa: F401\n\n\n" + "".join(moved)
open("src/click/params.py", "w").write(params)
core = "".join(rest)
if sys.argv[1] == "good":
    core = core.replace("\ndef __getattr__(", "\nfrom .params import _check_iter, Argument, Option, Parameter  # noqa: E402,F401\n\n\ndef __getattr__(", 1)
open(p, "w").write(core)
