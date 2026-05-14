import { useState, useEffect, useMemo } from 'react';

export interface VersionMetrics {
  version: number;
  date: string;
  successRate: number;
  avgLatency: number;
  p95Latency: number;
  avgTokens: number;
  executionCount: number;
}

export interface VersionComparison {
  versionA: number;
  versionB: number;
  successRateChange: number;
  latencyChange: number;
  tokenChange: number;
}

interface VersionTimelineProps {
  workflowId?: string;
  onVersionSelect?: (version: number) => void;
}

export function EvolutionTimeline({ workflowId, onVersionSelect }: VersionTimelineProps) {
  const [metrics, setMetrics] = useState<VersionMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedVersions, setSelectedVersions] = useState<[number, number]>([0, 0]);

  useEffect(() => {
    if (!workflowId) {
      setMetrics([]);
      setLoading(false);
      return;
    }

    fetchVersionHistory(workflowId).then(data => {
      setMetrics(data);
      setLoading(false);
    });
  }, [workflowId]);

  const handleVersionClick = (version: number, idx: number) => {
    const newSelect = [...selectedVersions] as [number, number];
    newSelect[idx] = version;
    setSelectedVersions(newSelect);
    onVersionSelect?.(version);
  };

  if (loading) {
    return <div className="evolution-timeline-loading">Loading timeline...</div>;
  }

  return (
    <div className="evolution-timeline">
      <h3>Version Evolution Timeline</h3>
      
      <div className="timeline-chart">
        {metrics.map((m) => (
          <div key={m.version} className="timeline-point" onClick={() => handleVersionClick(m.version, 0)}>
            <div className={`version-marker ${selectedVersions[0] === m.version ? 'selected-a' : ''} ${selectedVersions[1] === m.version ? 'selected-b' : ''}`}>
              v{m.version}
            </div>
            <div className="version-date">{m.date}</div>
            <div className="version-metric success-rate" title={`Success: ${(m.successRate * 100).toFixed(1)}%`}>
              {(m.successRate * 100).toFixed(0)}%
            </div>
            <div className="version-metric latency" title={`Latency: ${m.avgLatency.toFixed(0)}ms`}>
              {m.avgLatency.toFixed(0)}ms
            </div>
          </div>
        ))}
      </div>

      <div className="timeline-legend">
        <span className="legend-item"><span className="dot a"></span> Version A</span>
        <span className="legend-item"><span className="dot b"></span> Version B</span>
      </div>
    </div>
  );
}

export function VersionComparisonPanel({ versionA, versionB }: { versionA: number; versionB: number }) {
  const [comparison, setComparison] = useState<VersionComparison | null>(null);

  useEffect(() => {
    if (versionA && versionB) {
      fetchComparison(versionA, versionB).then(setComparison);
    }
  }, [versionA, versionB]);

  if (!comparison) return null;

  const formatChange = (value: number, isPositiveGood: boolean = true) => {
    const sign = value >= 0 ? '+' : '';
    const formatted = `${sign}${(value * 100).toFixed(1)}%`;
    const isGood = isPositiveGood ? value >= 0 : value <= 0;
    return <span className={isGood ? 'positive' : 'negative'}>{formatted}</span>;
  };

  return (
    <div className="version-comparison">
      <h4>Version {versionA} vs {versionB}</h4>
      <table>
        <tbody>
          <tr>
            <td>Success Rate</td>
            <td>{formatChange(comparison.successRateChange, true)}</td>
          </tr>
          <tr>
            <td>Latency</td>
            <td>{formatChange(comparison.latencyChange, false)}</td>
          </tr>
          <tr>
            <td>Token Usage</td>
            <td>{formatChange(comparison.tokenChange, false)}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}

export function VersionRollbackButton({ version, onConfirm }: { version: number; onConfirm: () => void }) {
  const [showConfirm, setShowConfirm] = useState(false);

  if (showConfirm) {
    return (
      <div className="rollback-confirm">
        <p>Rollback to version {version}?</p>
        <button className="btn-confirm" onClick={() => { onConfirm(); setShowConfirm(false); }}>
          Confirm Rollback
        </button>
        <button className="btn-cancel" onClick={() => setShowConfirm(false)}>
          Cancel
        </button>
      </div>
    );
  }

  return (
    <button className="btn-rollback" onClick={() => setShowConfirm(true)}>
      Rollback to v{version}
    </button>
  );
}

async function fetchVersionHistory(workflowId: string): Promise<VersionMetrics[]> {
  const response = await fetch(`/api/version-history/${workflowId}`);
  if (!response.ok) return [];
  return response.json();
}

async function fetchComparison(vA: number, vB: number): Promise<VersionComparison | null> {
  const response = await fetch(`/api/version-compare?vA=${vA}&vB=${vB}`);
  if (!response.ok) return null;
  return response.json();
}

export function ImpactDashboard() {
  const [impactData, setImpactData] = useState<ImpactReport[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchImpactData().then(data => {
      setImpactData(data);
      setLoading(false);
    });
  }, []);

  const totalSaved = useMemo(() => 
    impactData.reduce((sum, d) => sum + d.totalSaved, 0), 
    [impactData]
  );

  if (loading) return <div>Loading impact data...</div>;

  return (
    <div className="impact-dashboard">
      <div className="impact-summary">
        <h3>Optimization Value</h3>
        <div className="total-saved">${totalSaved.toFixed(2)}</div>
        <div className="period">Total saved this period</div>
      </div>

      <div className="impact-chart">
        {impactData.map(report => (
          <div key={report.date} className="impact-item">
            <div className="date">{report.date}</div>
            <div className="saved">${report.totalSaved.toFixed(2)}</div>
            <div className="changes">
              <span className={report.successRateChange >= 0 ? 'positive' : 'negative'}>
                {report.successRateChange >= 0 ? '+' : ''}{(report.successRateChange * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface ImpactReport {
  date: string;
  totalSaved: number;
  successRateChange: number;
  latencyChange: number;
  tokenReduction: number;
}

async function fetchImpactData(): Promise<ImpactReport[]> {
  const response = await fetch('/api/impact/weekly');
  if (!response.ok) return [];
  return response.json();
}

export default EvolutionTimeline;
