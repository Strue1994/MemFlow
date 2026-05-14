import { useEffect, useState } from 'react';

export function useMobileResponsive() {
  const [isMobile, setIsMobile] = useState(false);
  const [screenSize, setScreenSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const checkMobile = () => {
      const width = window.innerWidth;
      setIsMobile(width < 768);
      setScreenSize({ width, height: window.innerHeight });
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);

    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  return { isMobile, screenSize };
}

export function MobileNav() {
  const { isMobile } = useMobileResponsive();

  if (!isMobile) return null;

  const navItems = [
    { label: 'Workflows', icon: '📋', path: '/' },
    { label: 'Marketplace', icon: '🛒', path: '/marketplace' },
    { label: '+ New', icon: '➕', path: '/new' },
  ];

  return (
    <nav className="fixed bottom-0 left-0 right-0 bg-white border-t flex justify-around py-2 px-4 safe-area-pb">
      {navItems.map(item => (
        <a
          key={item.path}
          href={item.path}
          className="flex flex-col items-center text-xs text-gray-600"
        >
          <span className="text-lg">{item.icon}</span>
          <span>{item.label}</span>
        </a>
      ))}
    </nav>
  );
}

export function MobileWorkflowCard({ workflow, onClick }) {
  const { isMobile } = useMobileResponsive();

  return (
    <div 
      onClick={onClick}
      className={`${isMobile ? 'p-3' : 'p-4'} border rounded-lg hover:shadow-md transition-shadow`}
    >
      <h3 className="font-medium truncate">{workflow.name}</h3>
      <p className="text-gray-500 text-sm mt-1 line-clamp-2">
        {workflow.description}
      </p>
      <div className="flex justify-between items-center mt-2 text-xs text-gray-400">
        <span>v{workflow.version}</span>
        <span>{workflow.status}</span>
      </div>
    </div>
  );
}