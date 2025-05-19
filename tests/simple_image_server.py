# This is only for `real_repos_regression.py`. Do not use it otherwise.

from flask import Flask, request, send_file
app = Flask(__name__)

@app.route("/hung")
def hung():
    import time
    time.sleep(999999)

@app.route("/img/<any>")
def serve_image(any):
    empty = request.args.get("empty")

    if empty == "1":
        return send_file("./images/empty.png", mimetype="image/png")
    else:
        return send_file("./images/hello_world.webp", mimetype="image/webp")

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=12345)
