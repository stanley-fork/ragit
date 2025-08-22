import os
from server import spawn_ragit_server
import subprocess
import sys
from utils import goto_root

help_message = """
Commands
    run [--truncate-all]        It runs ragithub.
        [--port <n=8080>]
        [--backend <url>]
"""

if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else None

    if command == "run":
        args = [arg for arg in sys.argv[2:]]
        truncate = "--truncate-all" in args
        args = [arg for arg in args if arg != "--truncate-all"]
        port = None
        backend = None

        if "--port" in args:
            port_index = args.index("--port")
            port = int(args[port_index + 1])
            args = args[:port_index] + args[(port_index + 2):]

        if "--backend" in args:
            backend_index = args.index("--backend")
            backend = args[backend_index + 1]
            args = args[:backend_index] + args[(backend_index + 2):]

        if args != []:
            raise Exception(f"unknown arg: {args[0]}")

        ragithub_args = [] if port is None else ["--port", str(port)]

        if backend is None:
            spawn_ragit_server(truncate=truncate)

        else:
            ragithub_args += ["--backend", backend]

        goto_root()
        os.chdir("ragithub/frontend")
        subprocess.run(["cargo", "run", "--release", "--", *ragithub_args])
