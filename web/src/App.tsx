import React from 'react';
import { BrowserRouter, Routes, Route, NavLink } from 'react-router-dom';
import { Dashboard } from './pages/Dashboard';
import { Invoice } from './pages/Invoice';
import { Invoices } from './pages/Invoices';
import { HumanActions } from './pages/HumanActions';
import { Settings } from './pages/Settings';
import { LayoutDashboard, Receipt, FileText, UserCheck, Settings as SettingsIcon } from 'lucide-react';

export const App: React.FC = () => {
  return (
    <BrowserRouter>
      <div className="min-h-screen flex flex-col bg-slate-50 text-slate-900">
        <header className="bg-white border-b border-slate-200 sticky top-0 z-30">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="bg-blue-600 text-white font-bold px-2.5 py-1 rounded text-sm tracking-wider">TXFFP</div>
              <span className="font-bold text-slate-800 tracking-tight">票根自动开票 Agent</span>
            </div>

            <nav className="flex items-center gap-1">
              <NavLink
                to="/"
                end
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition ${
                    isActive ? 'bg-slate-100 text-blue-600' : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`
                }
              >
                <LayoutDashboard className="w-4 h-4" />
                仪表盘
              </NavLink>
              <NavLink
                to="/invoice"
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition ${
                    isActive ? 'bg-slate-100 text-blue-600' : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`
                }
              >
                <Receipt className="w-4 h-4" />
                开票
              </NavLink>
              <NavLink
                to="/invoices"
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition ${
                    isActive ? 'bg-slate-100 text-blue-600' : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`
                }
              >
                <FileText className="w-4 h-4" />
                历史发票
              </NavLink>
              <NavLink
                to="/human-actions"
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition ${
                    isActive ? 'bg-slate-100 text-blue-600' : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`
                }
              >
                <UserCheck className="w-4 h-4" />
                人工通道
              </NavLink>
              <NavLink
                to="/settings"
                className={({ isActive }) =>
                  `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition ${
                    isActive ? 'bg-slate-100 text-blue-600' : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50'
                  }`
                }
              >
                <SettingsIcon className="w-4 h-4" />
                设置
              </NavLink>
            </nav>
          </div>
        </header>

        <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/invoice" element={<Invoice />} />
            <Route path="/invoices" element={<Invoices />} />
            <Route path="/human-actions" element={<HumanActions />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
};
