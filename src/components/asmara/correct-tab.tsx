import { CheckCircle2, Loader2, Send, UserCog, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import {
  type CorrectionType,
  commands,
  type Episode,
  type GateResult,
  type SubmitResult_Serialize,
} from "@/lib/bindings";
import { formatAccuracy, humanize } from "@/lib/format";

const CORRECTION_TYPES: {
  value: CorrectionType;
  label: string;
  hint: string;
}[] = [
  {
    value: "fact_mapping",
    label: "Fact mapping",
    hint: "The facts were mapped wrong. Becomes a gated strategy item.",
  },
  {
    value: "rule_defect",
    label: "Rule defect",
    hint: "The symbolic rule is wrong. Routed to a human.",
  },
  {
    value: "vocabulary_gap",
    label: "Vocabulary gap",
    hint: "A missing slot/term. Routed to a human.",
  },
];

function GateVerdict({ gate }: { gate: GateResult }) {
  return (
    <Card className={gate.passed ? "border-primary/40" : "border-destructive/40"}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {gate.passed ? (
            <CheckCircle2 className="size-4 text-primary" />
          ) : (
            <XCircle className="size-4 text-destructive" />
          )}
          Verification gate: {gate.passed ? "PASS" : "FAIL"}
        </CardTitle>
        <CardDescription>
          Baseline {formatAccuracy(gate.baseline_accuracy)} → candidate{" "}
          {formatAccuracy(gate.candidate_accuracy)}
        </CardDescription>
      </CardHeader>
      {gate.reasons.length > 0 && (
        <CardContent>
          <p className="mb-1.5 font-medium text-muted-foreground text-xs">Reasons</p>
          <ul className="list-disc space-y-1 pl-5 text-sm">
            {gate.reasons.map((reason) => (
              <li key={reason} className="selectable">
                {reason}
              </li>
            ))}
          </ul>
        </CardContent>
      )}
    </Card>
  );
}

function SubmitOutcome({ result }: { result: SubmitResult_Serialize }) {
  if (result.routed === "human") {
    return (
      <Card className="border-primary/30">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <UserCog className="size-4 text-primary" />
            Queued for a human reviewer
          </CardTitle>
          <CardDescription>
            A {humanize(result.correction.type)} correction edits the symbolic moat — it is not
            auto-learned. It has been queued for expert review.
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {result.item?.gate ? (
        <GateVerdict gate={result.item.gate} />
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Lesson distilled</CardTitle>
            <CardDescription>The correction was routed through the gate.</CardDescription>
          </CardHeader>
        </Card>
      )}
      {result.item && (
        <p className="text-muted-foreground text-sm">
          Stored as a <Badge variant="secondary">{result.item.trust_state}</Badge> strategy item.
          Review it on the Review tab.
        </p>
      )}
    </div>
  );
}

export function CorrectTab() {
  const [episodes, setEpisodes] = useState<Episode[]>([]);
  const [episodeId, setEpisodeId] = useState("");
  const [type, setType] = useState<CorrectionType>("fact_mapping");
  const [decision, setDecision] = useState("");
  const [section, setSection] = useState("");
  const [expert, setExpert] = useState("hleb");
  const [note, setNote] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<SubmitResult_Serialize | null>(null);

  useEffect(() => {
    void commands
      .recentEpisodes(20)
      .then(setEpisodes)
      .catch(() => setEpisodes([]));
  }, []);

  const selectedType = CORRECTION_TYPES.find((entry) => entry.value === type);
  const canSubmit = Boolean(episodeId && expert.trim()) && !submitting;

  async function onSubmit() {
    if (!canSubmit) return;
    setSubmitting(true);
    setResult(null);
    try {
      const submission = await commands.submitCorrection({
        episode_id: episodeId,
        type,
        corrected_decision: decision.trim() || null,
        governing_section: section.trim() || null,
        expert: expert.trim(),
        note: note.trim() || null,
      });
      if (submission.status === "ok") {
        setResult(submission.data);
      }
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <UserCog className="size-4 text-primary" />
            Submit an expert correction
          </CardTitle>
          <CardDescription>
            Correction needs an external signal. Attach it to a real episode; a fact mapping passes
            the gate before it is ever learned.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="correct-episode">Episode</Label>
            <Select value={episodeId} onValueChange={setEpisodeId}>
              <SelectTrigger id="correct-episode" className="w-full">
                <SelectValue placeholder="Pick a recent episode…" />
              </SelectTrigger>
              <SelectContent>
                {episodes.map((episode) => (
                  <SelectItem key={episode.id} value={episode.id}>
                    <span className="line-clamp-1">
                      {episode.decision ? `${humanize(episode.decision)} — ` : ""}
                      {episode.question}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="correct-type">Correction type</Label>
              <Select value={type} onValueChange={(value) => setType(value as CorrectionType)}>
                <SelectTrigger id="correct-type" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CORRECTION_TYPES.map((entry) => (
                    <SelectItem key={entry.value} value={entry.value}>
                      {entry.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {selectedType && <p className="text-muted-foreground text-xs">{selectedType.hint}</p>}
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="correct-expert">Expert</Label>
              <Input
                id="correct-expert"
                value={expert}
                onChange={(event) => setExpert(event.target.value)}
                placeholder="who is correcting"
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="correct-decision">Corrected decision</Label>
              <Input
                id="correct-decision"
                value={decision}
                onChange={(event) => setDecision(event.target.value)}
                placeholder="e.g. reverse_charge_applies"
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="correct-section">Governing §</Label>
              <Input
                id="correct-section"
                value={section}
                onChange={(event) => setSection(event.target.value)}
                placeholder="e.g. §92e"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="correct-note">Note</Label>
            <Textarea
              id="correct-note"
              value={note}
              onChange={(event) => setNote(event.target.value)}
              placeholder="Why the original decision was wrong (optional)…"
              rows={3}
            />
          </div>

          <Separator />
          <Button onClick={onSubmit} disabled={!canSubmit}>
            {submitting ? <Loader2 className="size-4 animate-spin" /> : <Send />}
            {submitting ? "Submitting…" : "Submit correction"}
          </Button>
        </CardContent>
      </Card>

      {result && <SubmitOutcome result={result} />}
    </div>
  );
}
