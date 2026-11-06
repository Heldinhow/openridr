"""Drive the OpenRidr host bridge through setup -> trust -> restore -> analysis, offline.

Proves the bundled csharp-ls payload and its private .NET runtime are what run: the bridge has
OPENRIDR_BUNDLED_* pointing at the built app's Contents/Resources, the app home is empty, no
network is allowed and the only detected SDK root is a synthetic SDK-8-only root.
"""
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.request

PORT = int(os.environ.get("PORT", "4417"))
BASE = f"http://127.0.0.1:{PORT}"
SOLUTION = pathlib.Path(os.environ["SOLUTION"])
ERROR_FILE = pathlib.Path(os.environ["ERROR_FILE"])
TIMEOUT = int(os.environ.get("TIMEOUT", "420"))


def call(method, params=None):
    body = json.dumps({"method": method, "params": params or {}}).encode()
    request = urllib.request.Request(
        BASE + "/call", data=body, headers={"content-type": "application/json"}
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            payload = json.loads(response.read())
    except urllib.error.HTTPError as error:
        payload = json.loads(error.read())
    if payload.get("error"):
        raise RuntimeError(f"{method}: {payload['error']}")
    return payload.get("data", payload.get("result"))


def wait_health():
    deadline = time.time() + 60
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(BASE + "/health", timeout=5) as response:
                return json.loads(response.read())
        except Exception:
            time.sleep(0.3)
    raise SystemExit("host bridge never answered /health")


def until(label, predicate, timeout=TIMEOUT):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        last = call("workspace.state")
        if predicate(last):
            return last
        time.sleep(1)
    raise SystemExit(f"timed out waiting for {label}: {json.dumps(last)[:1200]}")


print("health:", wait_health())
opened = call("workspace.open", {"path": str(SOLUTION)})
print("opened:", opened["workspace"]["name"], "kind:", opened["workspace"]["kind"])
call("setup.start")

ready = until(
    "setup ready",
    lambda state: state["setup"]["state"] == "ready",
)
tool = ready["tools"]["csharp-ls"]
print("setup:", ready["setup"]["state"])
print("tool:", json.dumps({key: tool[key] for key in ["state", "bundled", "destination", "verifiedVersion", "sha256", "plan", "progress"]}))
print("runtime:", json.dumps({key: ready["sdk"]["host"][key] for key in ["state", "bundled", "version", "root"]}))
print("project sdk:", ready["sdk"]["project"]["state"], ready["sdk"]["project"]["requestedVersion"])
assert tool["state"] == "ready", tool
assert tool["bundled"] is True
assert tool["plan"] is None
assert ready["sdk"]["host"]["bundled"] is True

home = os.environ["OPENRIDR_HOME"]
print("app home tools/ exists:", pathlib.Path(home, "tools").exists())
assert not pathlib.Path(home, "tools").exists()

trusted = call("trust.grant")
print("trusted:", trusted["trust"]["granted"], "project sdk:", trusted["sdk"]["project"]["state"])
assert trusted["sdk"]["project"]["state"] == "resolved", trusted["sdk"]["project"]

call("restore.start")
restored = until(
    "restore ok",
    lambda state: state["restore"]["state"] == "ok",
)
print("restore:", restored["restore"]["state"], [project["state"] for project in restored["restore"]["projects"]])

call("lsp.start")
started = until(
    "lsp ready or partial",
    lambda state: state["lsp"]["state"] in ("ready", "partial", "failed"),
)
print("lsp:", started["lsp"]["state"], "| host root:", started["lsp"]["hostRoot"])
assert started["lsp"]["state"] in ("ready", "partial"), started["lsp"]

call("lsp.didOpen", {"path": str(ERROR_FILE)})
publication = until(
    "diagnostics for the error file",
    lambda state: any(
        (snapshot.get("path") or snapshot["uri"]).endswith(ERROR_FILE.name)
        and snapshot["items"]
        for snapshot in state["diagnostics"]["snapshots"]
    ),
)
entry = next(
    snapshot for snapshot in publication["diagnostics"]["snapshots"]
    if (snapshot.get("path") or snapshot["uri"]).endswith(ERROR_FILE.name)
    and snapshot["items"]
)
print("diagnostics:", json.dumps(entry["items"])[:600])
codes = [item.get("code") for item in entry["items"]]
assert "CS0029" in codes, codes

pids = subprocess.run(["pgrep", "-f", "CSharpLanguageServer.dll"], capture_output=True, text=True).stdout.split()
print("csharp-ls pids:", pids)
for pid in pids:
    listing = subprocess.run(["lsof", "-p", pid], capture_output=True, text=True).stdout
    first = [line for line in listing.splitlines() if "CSharpLanguageServer.dll" in line or "/dotnet " in line or line.rstrip().endswith("/dotnet")]
    print("pid", pid, ":", "\n".join(first[:3]))
print("home listing tools/:")
print(subprocess.run(["ls", "-la", home], capture_output=True, text=True).stdout)
print("RESULT: ok")
