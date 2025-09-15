import contextlib
import io
import json
import sys

def run(code, scope):
    with contextlib.redirect_stdout(io.StringIO()) as f:
        exec(code, scope)
    return f.getvalue()

inputs = json.loads(sys.stdin.read())
scope = {}
outputs = [run(code, scope) for code in inputs]

sys.stdout.write(json.dumps(outputs))
