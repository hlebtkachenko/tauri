import { ClipboardCheck, Library, ShieldCheck, Sparkles, UserCog } from "lucide-react";
import { AskTab } from "@/components/asmara/ask-tab";
import { CorrectTab } from "@/components/asmara/correct-tab";
import { ManageTab } from "@/components/asmara/manage-tab";
import { ReviewTab } from "@/components/asmara/review-tab";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useExternalLinks } from "@/hooks/use-external-links";

const TABS = [
  { value: "ask", label: "Ask", icon: Sparkles, Component: AskTab },
  { value: "correct", label: "Correct", icon: UserCog, Component: CorrectTab },
  {
    value: "review",
    label: "Review",
    icon: ClipboardCheck,
    Component: ReviewTab,
  },
  {
    value: "manage",
    label: "Manage knowledge",
    icon: Library,
    Component: ManageTab,
  },
] as const;

function App() {
  useExternalLinks();

  return (
    <Tabs defaultValue="ask" className="flex h-screen flex-col gap-0">
      <header className="flex shrink-0 flex-col gap-3 border-b px-6 py-4">
        <div className="flex items-center gap-2">
          <ShieldCheck className="size-5 text-primary" />
          <h1 className="font-semibold text-lg">Asmara</h1>
          <span className="text-muted-foreground text-sm">Fail-closed Czech-accounting expert</span>
        </div>
        <TabsList>
          {TABS.map(({ value, label, icon: Icon }) => (
            <TabsTrigger key={value} value={value}>
              <Icon />
              {label}
            </TabsTrigger>
          ))}
        </TabsList>
      </header>

      <ScrollArea className="min-h-0 flex-1">
        <main className="mx-auto w-full max-w-3xl px-6 py-6">
          {TABS.map(({ value, Component }) => (
            <TabsContent key={value} value={value}>
              <Component />
            </TabsContent>
          ))}
        </main>
      </ScrollArea>
    </Tabs>
  );
}

export default App;
