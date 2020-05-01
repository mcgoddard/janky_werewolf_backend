import websocket
try:
    import thread
except ImportError:
    import _thread as thread

class Player:
    name = ""
    received = []
    error = None
    opened = False
    ws = None
    thread = None

    def __init__(self, name, received, socket_url):
        self.name = name
        self.received = received
        websocket.enableTrace(True)
        self.ws = websocket.WebSocketApp(socket_url,
                                on_message = self.on_message,
                                on_error = self.on_error,
                                on_close = self.on_close)
        self.ws.on_open = self.on_open
        def run():
            self.ws.run_forever()
        self.thread = thread.start_new_thread(run, ())

    def on_message(self, ws, message):
        self.received.append(message)

    def on_error(self, ws, error):
        self.error = error

    def on_close(self, ws):
        self.opened = False

    def on_open(self, ws):
        self.opened = True

    def send(message):
        self.ws.send(message)

    def close():
        self.ws.close()
