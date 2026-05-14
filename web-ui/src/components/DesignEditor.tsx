import React, { useState } from 'react';
import Editor from '@monaco-editor/react';

interface DesignEditorProps {
  initialValue: any;
  onSave: (value: any) => void;
  onCancel: () => void;
}

export const DesignEditor: React.FC<DesignEditorProps> = ({ initialValue, onSave, onCancel }) => {
  const [code, setCode] = useState(JSON.stringify(initialValue, null, 2));
  const [error, setError] = useState<string | null>(null);

  const handleSave = () => {
    try {
      const parsed = JSON.parse(code);
      setError(null);
      onSave(parsed);
    } catch (e) {
      setError('JSON 格式错误: ' + (e instanceof Error ? e.message : String(e)));
    }
  };

  const handleFormat = () => {
    try {
      const parsed = JSON.parse(code);
      setCode(JSON.stringify(parsed, null, 2));
      setError(null);
    } catch (e) {
      setError('JSON 格式错误，无法格式化');
    }
  };

  return (
    <div className="border-2 border-blue-500 rounded-lg p-4 bg-white">
      <div className="flex justify-between items-center mb-2">
        <h3 className="text-lg font-semibold">编辑工作流设计</h3>
        <div className="flex space-x-2">
          <button 
            onClick={handleFormat}
            className="bg-gray-200 text-gray-700 px-3 py-1 rounded text-sm hover:bg-gray-300"
          >
            格式化 JSON
          </button>
        </div>
      </div>

      <div className="border rounded overflow-hidden mb-3">
        <Editor
          height="400px"
          defaultLanguage="json"
          value={code}
          onChange={(value) => setCode(value || '')}
          theme="vs-light"
          options={{
            minimap: { enabled: false },
            fontSize: 14,
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            automaticLayout: true,
          }}
        />
      </div>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-3 text-sm">
          {error}
        </div>
      )}

      <div className="flex justify-end space-x-2">
        <button 
          onClick={onCancel}
          className="bg-gray-500 text-white px-4 py-2 rounded hover:bg-gray-600"
        >
          取消
        </button>
        <button 
          onClick={handleSave}
          disabled={!!error}
          className={`px-4 py-2 rounded text-white ${error ? 'bg-gray-400 cursor-not-allowed' : 'bg-green-500 hover:bg-green-600'}`}
        >
          保存并继续
        </button>
      </div>
    </div>
  );
};

export default DesignEditor;