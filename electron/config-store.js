const fs = require('node:fs');
const path = require('node:path');

const DEFAULT_SETTINGS = {
  autoTranslate: true,
  targetLanguage: 'c',
  maxLinesPerTranslation: 3,
  context: {
    beforeChars: 1600,
    afterChars: 900,
  },
  ai: {
    provider: 'gemini',
    apiKey: '',
    model: 'gemini-2.5-flash',
  },
  editor: {
    tabSize: 4,
    insertSpaces: true,
    fontSize: 16,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    renderWhitespace: 'trailing',
    cursorStyle: 'line',
  },
  ui: {
    sidebarVisible: true,
    activityVisible: true,
    minimapEnabled: true,
    zoomLevel: 0,
  },
};

const deepMerge = (target, source) => {
  const output = { ...target };
  for (const [key, value] of Object.entries(source || {})) {
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      output[key] = deepMerge(target[key] || {}, value);
    } else {
      output[key] = value;
    }
  }
  return output;
};

function createConfigStore(app) {
  const filePath = path.join(app.getPath('userData'), 'gradatim-settings.json');

  const read = () => {
    try {
      if (!fs.existsSync(filePath)) {
        return { ...DEFAULT_SETTINGS };
      }
      const raw = fs.readFileSync(filePath, 'utf-8');
      const parsed = JSON.parse(raw);
      return deepMerge(DEFAULT_SETTINGS, parsed);
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  };

  let data = read();

  const write = next => {
    data = deepMerge(data, next);
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
    return data;
  };

  return {
    get: () => data,
    set: payload => write(payload),
    defaults: DEFAULT_SETTINGS,
  };
}

module.exports = { createConfigStore, DEFAULT_SETTINGS };

