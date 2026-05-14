import { useState } from 'react';
import { useLanguage } from '../lib/language';

interface MarketplaceCardProps {
  workflow: {
    id: string;
    name: string;
    description: string;
    rating: number;
    downloadCount: number;
    tags: string[];
  };
  onImport: (id: string) => void;
}

export function MarketplaceCard({ workflow, onImport }: MarketplaceCardProps) {
  const { text } = useLanguage();
  const [showPreview, setShowPreview] = useState(false);

  return (
    <div className="border rounded-lg p-4 hover:shadow-lg transition-shadow">
      <h3 className="font-bold text-lg">{workflow.name}</h3>
      <p className="text-gray-600 text-sm mt-1">{workflow.description}</p>
      
      <div className="flex items-center gap-2 mt-3">
        <span className="text-yellow-500">⭐ {workflow.rating.toFixed(1)}</span>
        <span className="text-gray-400">|</span>
        <span className="text-gray-500">{workflow.downloadCount} downloads</span>
      </div>
      
      <div className="flex gap-1 mt-2">
        {workflow.tags.map(tag => (
          <span key={tag} className="text-xs bg-gray-100 px-2 py-1 rounded">
            {tag}
          </span>
        ))}
      </div>
      
      <div className="flex gap-2 mt-4">
        <button
          onClick={() => setShowPreview(!showPreview)}
          className="flex-1 px-3 py-1 border rounded hover:bg-gray-50"
        >
          {showPreview ? text({ zh: '收起', en: 'Hide' }) : text({ zh: '预览', en: 'Preview' })}
        </button>
        <button
          onClick={() => onImport(workflow.id)}
          className="flex-1 px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          {text({ zh: '导入', en: 'Import' })}
        </button>
      </div>
      
      {showPreview && (
        <div className="mt-3 p-3 bg-gray-50 rounded text-xs font-mono">
          {text({ zh: '点击“导入”即可把这个工作流载入当前工作区。', en: 'Click “Import” to load this workflow into your workspace.' })}
        </div>
      )}
    </div>
  );
}

interface MarketplaceSearchProps {
  onSearch: (query: string) => void;
  onFilter: (tags: string[]) => void;
}

export function MarketplaceSearch({ onSearch, onFilter }: MarketplaceSearchProps) {
  const { text } = useLanguage();
  const [query, setQuery] = useState('');
  const [selectedTags, setSelectedTags] = useState<string[]>([]);

  const tags = ['HTTP', 'Database', 'AI', 'Scheduling', 'Notification'];

  const handleSearch = () => {
    onSearch(query);
  };

  const toggleTag = (tag: string) => {
    if (selectedTags.includes(tag)) {
      setSelectedTags(selectedTags.filter(t => t !== tag));
      onFilter(selectedTags.filter(t => t !== tag));
    } else {
      const newTags = [...selectedTags, tag];
      setSelectedTags(newTags);
      onFilter(newTags);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <input
          type="text"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
          placeholder={text({ zh: '搜索工作流…', en: 'Search workflows…' })}
          className="flex-1 px-4 py-2 border rounded"
        />
        <button
          onClick={handleSearch}
          className="px-4 py-2 bg-blue-500 text-white rounded"
        >
          {text({ zh: '搜索', en: 'Search' })}
        </button>
      </div>
      
      <div className="flex gap-2 flex-wrap">
        {tags.map(tag => (
          <button
            key={tag}
            onClick={() => toggleTag(tag)}
            className={`px-3 py-1 rounded text-sm ${
              selectedTags.includes(tag)
                ? 'bg-blue-500 text-white'
                : 'bg-gray-100 text-gray-700'
            }`}
          >
            {tag}
          </button>
        ))}
      </div>
    </div>
  );
}

export default function Marketplace() {
  const { text } = useLanguage();
  const demoWorkflows = [
    {
      id: 'demo-1',
      name: 'HTTP Trigger to Slack',
      description: 'A starter workflow that receives an HTTP event and posts a Slack message.',
      rating: 4.8,
      downloadCount: 128,
      tags: ['HTTP', 'Notification'],
    },
    {
      id: 'demo-2',
      name: 'Lead Intake to Sheets',
      description: 'Collect leads from forms and append them into Google Sheets.',
      rating: 4.6,
      downloadCount: 94,
      tags: ['Database', 'Scheduling'],
    },
  ];

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">{text({ zh: '市场', en: 'Marketplace' })}</h1>
        <p className="text-sm text-gray-500 mt-1">{text({ zh: '浏览可直接导入的起步工作流。', en: 'Browse starter workflows that can be imported directly.' })}</p>
      </div>

      <MarketplaceSearch onSearch={() => undefined} onFilter={() => undefined} />

      <div className="grid gap-4 md:grid-cols-2">
        {demoWorkflows.map((workflow) => (
          <MarketplaceCard
            key={workflow.id}
            workflow={workflow}
            onImport={() => {
              window.alert(text({ zh: `已导入工作流：${workflow.name}`, en: `Imported workflow: ${workflow.name}` }));
            }}
          />
        ))}
      </div>
    </div>
  );
}
