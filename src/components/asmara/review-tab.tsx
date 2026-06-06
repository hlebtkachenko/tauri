import { CheckCircle2, ClipboardCheck, Loader2, ShieldCheck, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { commands, type StoredStrategyItem_Serialize } from "@/lib/bindings";
import { formatAccuracy } from "@/lib/format";

const APPROVER = "hleb";

function ItemMeta({ item }: { item: StoredStrategyItem_Serialize }) {
  return (
    <div className="space-y-3 text-sm">
      <p className="selectable text-muted-foreground">{item.content}</p>
      {item.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {item.tags.map((tag) => (
            <Badge key={tag} variant="outline">
              {tag}
            </Badge>
          ))}
        </div>
      )}
      <p className="text-muted-foreground text-xs">
        Provenance: {item.provenance.source}
        {item.provenance.episode_id ? ` · episode ${item.provenance.episode_id}` : " · no episode"}
        {item.provenance.correction_id ? ` · correction ${item.provenance.correction_id}` : ""}
      </p>
    </div>
  );
}

function ProvisionalCard({
  item,
  onApproved,
}: {
  item: StoredStrategyItem_Serialize;
  onApproved: () => void;
}) {
  const [approving, setApproving] = useState(false);
  const passed = item.gate?.passed === true;

  async function onApprove() {
    setApproving(true);
    try {
      const result = await commands.approve(item.id, APPROVER);
      if (result.status === "ok") onApproved();
    } finally {
      setApproving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">{item.title}</CardTitle>
        <CardDescription>{item.description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <ItemMeta item={item} />
        <Separator />
        {item.gate ? (
          <div className="space-y-1.5">
            <div className="flex items-center gap-2 font-medium text-sm">
              {passed ? (
                <CheckCircle2 className="size-4 text-primary" />
              ) : (
                <XCircle className="size-4 text-destructive" />
              )}
              Gate {passed ? "PASS" : "FAIL"} · baseline{" "}
              {formatAccuracy(item.gate.baseline_accuracy)} → candidate{" "}
              {formatAccuracy(item.gate.candidate_accuracy)}
            </div>
            {item.gate.reasons.length > 0 && (
              <ul className="list-disc space-y-0.5 pl-5 text-muted-foreground text-xs">
                {item.gate.reasons.map((reason) => (
                  <li key={reason} className="selectable">
                    {reason}
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">No gate verdict recorded.</p>
        )}
        <Button onClick={onApprove} disabled={!passed || approving}>
          {approving ? <Loader2 className="size-4 animate-spin" /> : <ShieldCheck />}
          {passed ? "Approve (promote to trusted)" : "Blocked — gate did not pass"}
        </Button>
      </CardContent>
    </Card>
  );
}

function TrustedCard({ item }: { item: StoredStrategyItem_Serialize }) {
  return (
    <Card className="border-primary/30">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ShieldCheck className="size-4 text-primary" />
          {item.title}
        </CardTitle>
        <CardDescription>
          {item.description}
          {item.approved_by ? ` · approved by ${item.approved_by}` : ""}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ItemMeta item={item} />
      </CardContent>
    </Card>
  );
}

export function ReviewTab() {
  const [provisional, setProvisional] = useState<StoredStrategyItem_Serialize[]>([]);
  const [trusted, setTrusted] = useState<StoredStrategyItem_Serialize[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [prov, trust] = await Promise.all([
        commands.listStrategyItems("provisional"),
        commands.listStrategyItems("trusted"),
      ]);
      setProvisional(prov);
      setTrusted(trust);
    } catch {
      setProvisional([]);
      setTrusted([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <ClipboardCheck className="size-4 text-primary" />
          <h2 className="font-semibold">Provisional — awaiting approval</h2>
          <Badge variant="secondary">{provisional.length}</Badge>
        </div>
        {loading ? (
          <p className="text-muted-foreground text-sm">Loading…</p>
        ) : provisional.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            Nothing waiting. Provisional lessons appear here after a correction passes the gate.
          </p>
        ) : (
          <div className="space-y-4">
            {provisional.map((item) => (
              <ProvisionalCard key={item.id} item={item} onApproved={refresh} />
            ))}
          </div>
        )}
      </section>

      <Separator />

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <ShieldCheck className="size-4 text-primary" />
          <h2 className="font-semibold">Trusted</h2>
          <Badge variant="secondary">{trusted.length}</Badge>
        </div>
        {trusted.length === 0 ? (
          <p className="text-muted-foreground text-sm">No trusted items yet.</p>
        ) : (
          <div className="space-y-4">
            {trusted.map((item) => (
              <TrustedCard key={item.id} item={item} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
