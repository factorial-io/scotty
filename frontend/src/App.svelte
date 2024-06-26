<script>
    let websocket;
    let message = "";
    let messages = [];

    const connect = () => {
        websocket = new WebSocket("ws://localhost:8080/ws");

        websocket.onopen = () => {
            console.log("Connected to WebSocket server");
        };

        websocket.onmessage = (event) => {
            messages = [...messages, event.data];
        };

        websocket.onclose = () => {
            console.log("Disconnected from WebSocket server");
        };

        websocket.onerror = (error) => {
            console.error("WebSocket error:", error);
        };
    };

    const sendMessage = () => {
        if (websocket && websocket.readyState === WebSocket.OPEN) {
            websocket.send(message);
            message = "";
        } else {
            console.error("WebSocket is not open");
        }
    };
</script>

<main>
    <h1>WebSocket Example</h1>

    <button on:click={connect}>Connect</button>

    <div class="messages">
        {#each messages as msg}
            <div>{msg}</div>
        {/each}
    </div>

    <input type="text" bind:value={message} placeholder="Enter message" />
    <button on:click={sendMessage}>Send</button>
</main>

<style>
    .messages {
        border: 1px solid #ccc;
        padding: 10px;
        height: 200px;
        overflow-y: auto;
    }
</style>
