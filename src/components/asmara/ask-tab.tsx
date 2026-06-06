import { Channel } from "@tauri-apps/api/core";
import { Loader2, ScaleIcon, ShieldAlert, Sparkles } from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { commands, type Outcome } from "@/lib/bindings";
import { humanize } from "@/lib/format";

// The progress Channel streams one of these stage tokens per step.
const STAGES = ["route", "retrieve", "extract", "solve", "gate"] as const;

function StageTrail({ active }: { active: string | null }) {
  const reached = active ? STAGES.indexOf(active as (typeof STAGES)[number]) : -1;
  return (
    <div className="flex flex-wrap items-center gap-2">
      {STAGES.map((stage, index) => {
        const isDone = reached > index;
        const isCurrent = reached === index;
        return (
          <Badge
            key={stage}
            variant={isCurrent ? "default" : isDone ? "secondary" : "outline"}
            className="capitalize"
          >
            {isCurrent && <Loader2 className="size-3 animate-spin" />}
            {stage}
          </Badge>
        );
      })}
    </div>
  );
}

function AnswerCard({ outcome }: { outcome: Extract<Outcome, { kind: "answer" }> }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ScaleIcon className="size-4 text-primary" />
          {humanize(outcome.decision)}
        </CardTitle>
        <CardDescription>Decision reached and verified by the gate.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {outcome.citations.length > 0 && (
          <div className="space-y-1.5">
            <p className="font-medium text-muted-foreground text-xs">Governing sections</p>
            <div className="flex flex-wrap gap-1.5">
              {outcome.citations.map((cite) => (
                <Badge key={cite} variant="secondary" className="font-mono">
                  {cite}
                </Badge>
              ))}
            </div>
          </div>
        )}
        {outcome.justification.length > 0 && (
          <>
            <Separator />
            <div className="space-y-1.5">
              <p className="font-medium text-muted-foreground text-xs">Justification</p>
              <ul className="list-disc space-y-1 pl-5 text-sm">
                {outcome.justification.map((line, index) => (
                  // Justification lines have no stable id; index is stable here.
                  // biome-ignore lint/suspicious/noArrayIndexKey: ordered static list
                  <li key={index} className="selectable">
                    {line}
                  </li>
                ))}
              </ul>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function AbstainCard({ outcome }: { outcome: Extract<Outcome, { kind: "abstain" }> }) {
  return (
    <Card className="border-destructive/40">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-destructive">
          <ShieldAlert className="size-4" />I don&apos;t know — escalate to an expert
        </CardTitle>
        <CardDescription>
          This case is outside what can be answered safely. It was refused, not guessed.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-1.5">
          <p className="font-medium text-muted-foreground text-xs">Reason</p>
          <p className="selectable text-sm">{outcome.reason}</p>
        </div>
      </CardContent>
    </Card>
  );
}

export function AskTab() {
  const [question, setQuestion] = useState("");
  const [stage, setStage] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  async function onAsk() {
    const trimmed = question.trim();
    if (!trimmed || running) return;
    setRunning(true);
    setOutcome(null);
    setStage(null);

    const channel = new Channel<string>();
    channel.onmessage = (next) => setStage(next);

    const result = await commands.ask(trimmed, null, channel);
    // Fail-closed: the engine never returns Err in practice, but if the IPC
    // boundary ever surfaces one we present an honest abstain, never a raw error.
    if (result.status === "ok") {
      setOutcome(result.data);
    } else {
      setOutcome({
        kind: "abstain",
        reason: "The engine could not complete this request. Escalate to an expert.",
      });
    }
    setStage(null);
    setRunning(false);
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="size-4 text-primary" />
            Ask a regulated question
          </CardTitle>
          <CardDescription>
            Describe the case. The engine routes, retrieves, extracts, solves, then gates the
            answer. It refuses rather than guesses.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="ask-case">Case</Label>
            <Textarea
              id="ask-case"
              value={question}
              onChange={(event) => setQuestion(event.target.value)}
              placeholder="e.g. A Czech VAT payer supplies construction work to another Czech VAT payer…"
              rows={6}
              disabled={running}
            />
          </div>
          <div className="flex items-center gap-3">
            <Button onClick={onAsk} disabled={running || !question.trim()}>
              {running ? <Loader2 className="size-4 animate-spin" /> : <Sparkles />}
              {running ? "Working…" : "Ask"}
            </Button>
            {running && <StageTrail active={stage} />}
          </div>
        </CardContent>
      </Card>

      {outcome?.kind === "answer" && <AnswerCard outcome={outcome} />}
      {outcome?.kind === "abstain" && <AbstainCard outcome={outcome} />}
    </div>
  );
}
