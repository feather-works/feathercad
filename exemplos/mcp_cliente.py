"""Cliente mínimo do servidor MCP do FeatherCAD (JSON-RPC por stdio), para dirigir o binário novo."""
import json, subprocess

EXE = "/run/media/maragon/Arquivos/GitHub/FeatherOffice/feathercad/crates/feathercad-mcp/target/release/feathercad-mcp"


class Mcp:
    def __init__(self, exe=EXE):
        self.p = subprocess.Popen([exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.n = 0
        self.pedir("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "gerador", "version": "1"}})

    def pedir(self, metodo, params):
        self.n += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": metodo, "params": params}) + "\n")
        self.p.stdin.flush()
        r = json.loads(self.p.stdout.readline())
        if "error" in r:
            raise RuntimeError(f"{metodo}: {r['error']}")
        return r["result"]

    def call(self, nome, args=None):
        r = self.pedir("tools/call", {"name": nome, "arguments": args or {}})
        txt = r["content"][0]["text"] if r["content"] and r["content"][0]["type"] == "text" else ""
        if r["isError"]:
            raise RuntimeError(f"{nome} {json.dumps(args, ensure_ascii=False)[:200]}: {txt}")
        try:
            return json.loads(txt)
        except json.JSONDecodeError:
            return txt

    def fechar(self):
        self.p.kill()
