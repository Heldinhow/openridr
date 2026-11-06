"""Breakpoint/step proof over the host bridge with the bundled netcoredbg + bundled analysis."""
import json, os, time, urllib.request, urllib.error

BASE = "http://127.0.0.1:4547"
WORK = "/tmp/openridr-bundled-sim/work/OrderDesk"
PROGRAM = f"{WORK}/src/OrderDesk.Cli/Program.cs"
TIMEOUT = int(os.environ.get("TIMEOUT", "600"))


def call(method, params=None):
    body = json.dumps({"method": method, "params": params or {}}).encode()
    request = urllib.request.Request(BASE + "/call", data=body, headers={"content-type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=120) as response:
            payload = json.loads(response.read())
    except urllib.error.HTTPError as error:
        payload = json.loads(error.read())
    if payload.get("error"):
        raise RuntimeError(f"{method}: {payload['error']}")
    return payload.get("data", payload.get("result"))


def until(label, predicate, timeout=TIMEOUT):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = call("workspace.state")
        if predicate(last):
            return last
        time.sleep(1)
    raise SystemExit(f"timed out waiting for {label}: {json.dumps(last)[:1500]}")


try:
    call("debug.stop")
except Exception as error:  # no session to stop
    print("debug.stop:", error)
call("evaluation.run", {"configuration": "Debug"})
state = until("evaluation ok", lambda state: state["evaluation"]["state"] == "ok")
print("evaluation:", state["evaluation"]["state"], "context:", state["evaluation"]["context"])

state = call("debug.setBreakpoints", {"path": PROGRAM, "lines": [4]})
print("breakpoints:", json.dumps(state["dap"]["breakpoints"]))

call("debug.start", {"project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj"})
state = until("build + breakpoint", lambda state: state["dap"]["state"] in ("paused", "failed", "exited"))
print("dap after start:", state["dap"]["state"], "| stopped:", json.dumps(state["dap"].get("stopped"))[:200], "| adapter:", state["debug"]["installedBinary"])
if state["dap"]["state"] != "paused":
    raise SystemExit("breakpoint was not hit: " + json.dumps(state["dap"])[:1500])

state = until("frame line 4", lambda state: any(frame.get("line") == 4 for frame in state["dap"]["frames"]))
frames = state["dap"]["frames"]
print("frames:", json.dumps(frames)[:400])
frame_id = next(frame["id"] for frame in frames if frame.get("line") == 4)
variables = call("debug.variables", {"frameId": frame_id})
print("variables:", json.dumps(variables["dap"].get("locals"))[:600])

call("debug.continue")
finished = until("debuggee output", lambda state: "Order A-104" in state["dap"]["output"])
print("debug output tail:", finished["dap"]["output"][-400:].replace("\n", " | "))
state = until("session ended", lambda state: state["dap"]["state"] in ("exited", "terminated", "idle"))
print("dap final:", state["dap"]["state"], "exit:", state["dap"].get("exit"))
print("RESULT: ok")
