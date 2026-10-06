#!/usr/bin/env python3
"""Starts HotDogStand and uses it as a person does, through the MCP server of
Slint. It needs a binary that is built with the `slint/mcp` feature:

    SLINT_EMIT_DEBUG_INFO=1 cargo build -p hotdogstand --features slint/mcp
    python3 scripts/smoke_test.py target/debug/hotdogstand --out smoke

It makes a new workspace in a temporary directory, adds a ticket, opens it,
adds a comment, and writes a screenshot of each window to the directory of
--out. It stops with an error at the first step that fails.

Set --headless to use the headless backend of Slint, for a computer with no
display, such as a CI runner.

The module also holds the small MCP client that the step functions use, so a
person can import it to drive the application by hand.
"""

import argparse
import base64
import http.client
import json
import os
import socket
import subprocess
import sys
import tempfile
import time

PORT = 9315


class Mcp:
    """A client for the MCP server that a Slint application starts."""

    def __init__(self, port=PORT):
        self.port = port
        self.next_id = 1

    def call(self, tool, arguments=None):
        body = json.dumps({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments or {}},
        })
        self.next_id += 1
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=30)
        try:
            connection.request("POST", "/mcp", body, {
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
                "Connection": "close",
            })
            text = connection.getresponse().read().decode()
        finally:
            connection.close()
        # The server can answer as an event stream.
        if not text.lstrip().startswith("{"):
            text = "".join(line[5:] for line in text.splitlines() if line.startswith("data:"))
        reply = json.loads(text)
        if "error" in reply:
            raise RuntimeError(f"{tool}: {reply['error']}")
        result = reply["result"]
        if result.get("isError"):
            raise RuntimeError(f"{tool}: {result.get('content')}")
        return result.get("content", [])

    def data(self, tool, arguments=None):
        """Calls a tool and gives the JSON in its first text answer."""
        for item in self.call(tool, arguments):
            if item.get("type") == "text":
                return json.loads(item["text"])
        return None

    def windows(self):
        return self.data("list_windows")["windowHandles"]

    def find(self, window, role):
        """Gives (handle, label) for each element of an accessible role."""
        props = self.data("get_window_properties", {"windowHandle": window})
        found = self.data("query_element_descendants", {
            "elementHandle": props["rootElementHandle"],
            "findAll": True,
            "queryStack": [{"matchDescendants": True}, {"matchElementAccessibleRole": role}],
        })
        handles = found.get("elementHandles", []) if isinstance(found, dict) else found
        result, seen = [], set()
        for handle in handles:
            if handle["index"] in seen:
                continue
            seen.add(handle["index"])
            element = self.data("get_element_properties", {"elementHandle": handle})
            label = element.get("accessibleLabel") or element.get("accessibleValue") or ""
            result.append((handle, label))
        return result

    def element(self, window, role, label):
        for handle, text in self.find(window, role):
            if text == label:
                return handle
        raise RuntimeError(f"no {role} with the label {label!r}")

    def press(self, handle, action="Default_"):
        self.call("invoke_accessibility_action", {"elementHandle": handle, "action": action})

    def type_into(self, handle, text):
        self.call("set_element_value", {"elementHandle": handle, "value": text})

    def screenshot(self, window, path):
        for item in self.call("take_screenshot", {"windowHandle": window}):
            if item.get("type") == "image":
                with open(path, "wb") as f:
                    f.write(base64.b64decode(item["data"]))
                return
        raise RuntimeError("the screenshot gave no image")


def wait_for_port(port, process, seconds=60):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if process.poll() is not None:
            raise RuntimeError(f"the application stopped with the code {process.returncode}")
        with socket.socket() as s:
            s.settimeout(0.2)
            if s.connect_ex(("127.0.0.1", port)) == 0:
                return
        time.sleep(0.1)
    raise RuntimeError("the MCP server did not open its port")


def wait_for(condition, what, seconds=10):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = condition()
        if value:
            return value
        time.sleep(0.2)
    raise RuntimeError(f"timed out: {what}")


def step(text):
    print(f"smoke: {text}", flush=True)


def run(mcp, out):
    main = wait_for(lambda: mcp.windows(), "the main window")[0]
    step("the main window is open")
    mcp.screenshot(main, os.path.join(out, "1-empty.png"))

    step("make a new ticket")
    mcp.press(mcp.element(main, "Button", "New ticket"))
    ticket = wait_for(lambda: mcp.windows()[1:], "the window of the new ticket")[-1]
    title = mcp.find(ticket, "TextInput")[1][0]
    mcp.type_into(title, "The printer is on fire")
    mcp.press(mcp.element(ticket, "Button", "Create"))

    step("the list shows the ticket")
    rows = wait_for(lambda: mcp.find(main, "ListItem"), "a row in the list")
    if rows[0][1] != "1 The printer is on fire":
        raise RuntimeError(f"the list shows {rows[0][1]!r}")
    mcp.screenshot(main, os.path.join(out, "2-list.png"))

    step("open the ticket and add a comment")
    before = len(mcp.windows())
    mcp.press(rows[0][0])
    opened = wait_for(lambda: mcp.windows()[before:], "the window of the ticket")[-1]
    mcp.press(mcp.element(opened, "Tab", "History"))
    comment = wait_for(lambda: mcp.find(opened, "TextInput"), "the comment field")[-1][0]
    mcp.type_into(comment, "It **burns**.")
    mcp.press(mcp.element(opened, "Button", "Add comment"))
    wait_for(lambda: any(label == "Edit the comment" for _, label in mcp.find(opened, "Button")),
             "the comment in the history")
    mcp.screenshot(opened, os.path.join(out, "3-history.png"))

    step("close the ticket")
    mcp.press(mcp.element(opened, "Button", "Close ticket"))
    wait_for(lambda: not mcp.find(main, "ListItem"), "the closed ticket leaves the list")
    step("all steps passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("binary", help="a HotDogStand binary built with the slint/mcp feature")
    parser.add_argument("--out", default="smoke", help="the directory for the screenshots")
    parser.add_argument("--headless", action="store_true", help="use the headless backend")
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)

    with tempfile.TemporaryDirectory() as home:
        env = dict(os.environ,
                   HOTDOGSTAND_WORKSPACE=os.path.join(home, "workspace.db"),
                   SLINT_MCP_PORT=str(PORT))
        if args.headless:
            env["SLINT_BACKEND"] = "headless"
        log = open(os.path.join(args.out, "app.log"), "w")
        process = subprocess.Popen([args.binary], env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            wait_for_port(PORT, process)
            run(Mcp(), args.out)
        except Exception as error:
            print(f"smoke: FAILED: {error}", file=sys.stderr)
            return 1
        finally:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
            log.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
