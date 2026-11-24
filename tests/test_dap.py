#!/usr/bin/env python3
import json
import subprocess
import sys

def send_msg(proc, obj):
    body = json.dumps(obj)
    msg = f"Content-Length: {len(body)}\r\n\r\n{body}"
    proc.stdin.write(msg.encode("utf-8"))
    proc.stdin.flush()

def read_msg(proc):
    header = proc.stdout.readline().decode("utf-8")
    if not header:
        return None
    while header.strip() == "":
        header = proc.stdout.readline().decode("utf-8")
        if not header:
            return None
    
    assert header.startswith("Content-Length:"), f"Invalid header: {header}"
    length = int(header.split(":")[1].strip())
    
    # Read blank line
    blank = proc.stdout.readline()
    
    body = proc.stdout.read(length).decode("utf-8")
    return json.loads(body)

def main():
    print("Testing Syntra DAP Server...")
    proc = subprocess.Popen(
        ["./target/debug/syn", "debug", "--dap"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    # 1. Initialize
    send_msg(proc, {
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {"adapterID": "syntra"}
    })
    init_resp = read_msg(proc)
    print("Initialize Response:", init_resp)
    assert init_resp["command"] == "initialize" and init_resp["success"] is True

    init_ev = read_msg(proc)
    print("Initialized Event:", init_ev)
    assert init_ev["event"] == "initialized"

    # 2. Launch
    send_msg(proc, {
        "seq": 2,
        "type": "request",
        "command": "launch",
        "arguments": {
            "program": "examples/test_debug.syn",
            "stopOnEntry": False
        }
    })
    launch_resp = read_msg(proc)
    print("Launch Response:", launch_resp)
    assert launch_resp["command"] == "launch" and launch_resp["success"] is True

    # 3. SetBreakpoints (at line 3)
    send_msg(proc, {
        "seq": 3,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {"path": "examples/test_debug.syn"},
            "breakpoints": [{"line": 3}]
        }
    })
    bp_resp = read_msg(proc)
    print("SetBreakpoints Response:", bp_resp)
    assert bp_resp["command"] == "setBreakpoints" and bp_resp["success"] is True
    assert bp_resp["body"]["breakpoints"][0]["verified"] is True

    # 4. ConfigurationDone
    send_msg(proc, {
        "seq": 4,
        "type": "request",
        "command": "configurationDone"
    })
    conf_resp = read_msg(proc)
    print("ConfigurationDone Response:", conf_resp)
    assert conf_resp["command"] == "configurationDone" and conf_resp["success"] is True

    # Expect stopped event (breakpoint hit at line 3)
    stop_ev = read_msg(proc)
    print("Stopped Event:", stop_ev)
    assert stop_ev["event"] == "stopped"
    assert stop_ev["body"]["reason"] == "breakpoint"

    # 5. Threads
    send_msg(proc, {
        "seq": 5,
        "type": "request",
        "command": "threads"
    })
    threads_resp = read_msg(proc)
    print("Threads Response:", threads_resp)
    assert len(threads_resp["body"]["threads"]) > 0

    # 6. StackTrace
    send_msg(proc, {
        "seq": 6,
        "type": "request",
        "command": "stackTrace",
        "arguments": {"threadId": 1}
    })
    st_resp = read_msg(proc)
    print("StackTrace Response:", st_resp)
    frames = st_resp["body"]["stackFrames"]
    assert len(frames) >= 2
    assert frames[0]["name"] == "compute"
    assert frames[0]["line"] == 3
    assert frames[1]["name"] == "main"

    # 7. Scopes & Variables
    send_msg(proc, {
        "seq": 7,
        "type": "request",
        "command": "scopes",
        "arguments": {"frameId": 0}
    })
    scopes_resp = read_msg(proc)
    print("Scopes Response:", scopes_resp)

    send_msg(proc, {
        "seq": 8,
        "type": "request",
        "command": "variables",
        "arguments": {"variablesReference": 1000}
    })
    vars_resp = read_msg(proc)
    print("Variables Response:", vars_resp)
    vars_map = {v["name"]: v["value"] for v in vars_resp["body"]["variables"]}
    assert vars_map.get("a") == "5"
    assert vars_map.get("b") == "7"
    assert vars_map.get("x") == "10"

    # 8. Evaluate expression in paused frame
    send_msg(proc, {
        "seq": 9,
        "type": "request",
        "command": "evaluate",
        "arguments": {"expression": "x * 100", "frameId": 0}
    })
    eval_resp = read_msg(proc)
    print("Evaluate Response:", eval_resp)
    assert eval_resp["body"]["result"] == "1000"

    # 9. Next (StepOver)
    send_msg(proc, {
        "seq": 10,
        "type": "request",
        "command": "next",
        "arguments": {"threadId": 1}
    })
    next_resp = read_msg(proc)
    print("Next Response:", next_resp)
    step_ev = read_msg(proc)
    print("Step Event:", step_ev)
    assert step_ev["event"] == "stopped"

    # Check variables after step: 'y' should now be present
    send_msg(proc, {
        "seq": 11,
        "type": "request",
        "command": "variables",
        "arguments": {"variablesReference": 1000}
    })
    vars_resp2 = read_msg(proc)
    print("Variables after step:", vars_resp2)
    vars_map2 = {v["name"]: v["value"] for v in vars_resp2["body"]["variables"]}
    assert vars_map2.get("y") == "17"

    # 10. Continue
    send_msg(proc, {
        "seq": 12,
        "type": "request",
        "command": "continue",
        "arguments": {"threadId": 1}
    })
    cont_resp = read_msg(proc)
    print("Continue Response:", cont_resp)
    term_ev = read_msg(proc)
    print("Terminated Event:", term_ev)
    assert term_ev["event"] == "terminated"

    # 11. Disconnect
    send_msg(proc, {
        "seq": 13,
        "type": "request",
        "command": "disconnect"
    })
    disc_resp = read_msg(proc)
    print("Disconnect Response:", disc_resp)

    proc.wait(timeout=3)
    print("\n✅ DAP SERVER PROTOCOL VERIFICATION PASSED 100%!")

if __name__ == "__main__":
    main()
