from flask import Flask

app = Flask(__name__)

@app.route("/api/serverlist")
def serverlist():
    return {
        "cicadagrad": {
            "ip": "cicada.kattmys.se:1997",
            "description": "The main kattmys chat server"
        }
    }


if __name__ == "__main__":
    app.run("localhost", port=1212)
