import { Button } from "@/components/ui/button";
import { useExternalLinks } from "@/hooks/use-external-links";

function App() {
  useExternalLinks();

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4">
      <h1 className="font-semibold text-2xl">Tauri Starter</h1>
      <Button>It works</Button>
    </main>
  );
}

export default App;
