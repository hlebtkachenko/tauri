import { History, Library } from "lucide-react";
import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { commands, type Episode, type Topic } from "@/lib/bindings";
import { formatTimestamp, humanize } from "@/lib/format";

function statusVariant(status: string): "default" | "secondary" | "destructive" | "outline" {
  if (status === "abstain") return "destructive";
  if (status === "answer") return "default";
  return "secondary";
}

export function ManageTab() {
  const [topics, setTopics] = useState<Topic[]>([]);
  const [episodes, setEpisodes] = useState<Episode[]>([]);

  useEffect(() => {
    void commands
      .listTopics()
      .then(setTopics)
      .catch(() => setTopics([]));
    void commands
      .recentEpisodes(10)
      .then(setEpisodes)
      .catch(() => setEpisodes([]));
  }, []);

  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <Library className="size-4 text-primary" />
          <h2 className="font-semibold">Topics</h2>
          <Badge variant="secondary">{topics.length}</Badge>
        </div>
        {topics.length === 0 ? (
          <p className="text-muted-foreground text-sm">No topics registered.</p>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {topics.map((topic) => (
              <Card key={topic.id}>
                <CardHeader>
                  <CardTitle className="font-mono text-sm">{topic.id}</CardTitle>
                  <CardDescription>{topic.description}</CardDescription>
                </CardHeader>
                {topic.tags.length > 0 && (
                  <CardContent>
                    <div className="flex flex-wrap gap-1.5">
                      {topic.tags.map((tag) => (
                        <Badge key={tag} variant="outline">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                  </CardContent>
                )}
              </Card>
            ))}
          </div>
        )}
      </section>

      <Separator />

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <History className="size-4 text-primary" />
          <h2 className="font-semibold">Recent episodes</h2>
          <Badge variant="secondary">{episodes.length}</Badge>
        </div>
        {episodes.length === 0 ? (
          <p className="text-muted-foreground text-sm">No episodes recorded yet.</p>
        ) : (
          <div className="space-y-3">
            {episodes.map((episode) => (
              <Card key={episode.id}>
                <CardHeader>
                  <div className="flex items-start justify-between gap-3">
                    <CardTitle className="text-base">
                      {episode.decision ? humanize(episode.decision) : "No decision"}
                    </CardTitle>
                    <Badge variant={statusVariant(episode.status)} className="capitalize">
                      {episode.status}
                    </Badge>
                  </div>
                  <CardDescription className="selectable">{episode.question}</CardDescription>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-muted-foreground text-xs">
                    {episode.topic && <span>Topic: {episode.topic}</span>}
                    <span>As of: {episode.as_of_date}</span>
                    <span>{formatTimestamp(episode.created_at)}</span>
                  </div>
                  {episode.citations.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {episode.citations.map((cite) => (
                        <Badge key={cite} variant="secondary" className="font-mono">
                          {cite}
                        </Badge>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
