import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useExternalLinks } from "@/hooks/use-external-links";
import { commands } from "@/lib/bindings";

function App() {
  useExternalLinks();
  const [message, setMessage] = useState("");

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4">
      <h1 className="font-semibold text-2xl">Tauri Starter</h1>
      <Button onClick={async () => setMessage(await commands.greet("Tauri"))}>
        Greet from Rust
      </Button>
      {message && <p className="text-muted-foreground text-sm">{message}</p>}
    </main>
  );
}

export default App;
