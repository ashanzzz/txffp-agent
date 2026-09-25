import React, { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, NavLink, useLocation } from 'react-router-dom';
import { Dashboard } from './pages/Dashboard';
import { Invoice } from './pages/Invoice';
import { Invoices } from './pages/Invoices';
import { HumanActions } from './pages/HumanActions';
import { Settings } from './pages/Settings';
import { api, AuthState } from './api/client';
import {
  LayoutDashboard,
  Receipt,
  FileText,
  UserCheck,
  Settings as SettingsIcon,
  Compass,
  LogIn,
  Lock,
  ShieldAlert,
  Loader2,
} from 'lucide-react';

interface AuthContextType {
  auth: AuthState | null;
  loading: boolean;
  refreshAuth: () => Promise<void>;
  handleInitBrowser: () => Promise<void>;
  handleLogin: () => Promise<void>;
}

export const AuthContext = React.createContext<AuthContextType>({
  auth: null,
  loading: false,
  refreshAuth: async () => {},
  handleInitBrowser: async () => {},
  handleLogin: async () => {},
});

// Auth Guard Component: Blocks access to protected pages unless logged in
const AuthGuard: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { auth, loading, handleLogin, handleInitBrowser } = React.useContext(AuthContext);
  const location = useLocation();

  if (loading && !auth) {
    return (
      <div className="flex flex-col items-center justify-center p-12 text-slate-500 space-y-3">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
        <p className="text-sm">正在核验票根会话状态...</p>
      </div>
    );
  }

  const isLoggedIn = auth?.status === 'LOGGED_IN';

  if (!isLoggedIn) {
    return (
      <div className="max-w-xl mx-auto my-8 bg-white border border-slate-200 rounded-xl p-8 text-center shadow-sm space-y-5">
        <div className="w-14 h-14 bg-amber-50 text-amber-600 rounded-full flex items-center justify-center mx-auto">
          <Lock className="w-7 h-7" />
        </div>
        <div>
          <h2 className="text-xl font-bold text-slate-900">需要先登录票根网账户</h2>
          <p className="text-slate-500 text-sm mt-2 leading-relaxed">
            您当前正在访问受保护的业务模块（{location.pathname === '/invoice' ? '开票与通行记录' : '历史发票'}）。
            由于通行数据与发票信息属于敏感业务，必须在票根网登录鉴权成功后方可使用。
          </p>
        </div>

        <div className="bg-slate-50 border border-slate-100 rounded-lg p-4 text-xs text-slate-600 text-left space-y-1.5">
          <div className="flex items-center gap-2 font-semibold text-slate-800">
            <ShieldAlert className="w-4 h-4 text-amber-500" />
            快速操作指引：
          </div>
          <div>1. 点击“一键登录账户”，系统将自动驱动 Steel 浏览器执行免验证登录流程；</div>
          <div>2. 若首次启动或浏览器未就绪，可先点击“初始化浏览器”建立远程通道。</div>
        </div>

        <div className="flex items-center justify-center gap-3 pt-2">
          <button
            onClick={handleInitBrowser}
            className="flex items-center gap-2 border border-slate-200 hover:bg-slate-50 text-slate-700 px-4 py-2 rounded-md text-sm font-medium transition"
          >
            <Compass className="w-4 h-4 text-blue-600" />
            初始化浏览器
          </button>
          <button
            onClick={handleLogin}
            className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-5 py-2 rounded-md text-sm font-medium shadow-sm transition"
          >
            <LogIn className="w-4 h-4" />
            一键登录账户
          </button>
        </div>
      </div>
    );
  }

  return <>{children}</>;
};

export const AppContent: React.FC = () => {
  const [auth, setAuth] = useState<AuthState | null>(null);
  const [loading, setLoading] = useState(false);
  const [browserMsg, setBrowserMsg] = useState<string | null>(null);

  const refreshAuth = async () => {
    try {
      const res = await api.getAuthStatus();
      if (res.ok && res.data) {
        setAuth(res.data);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleInitBrowser = async () => {
    setLoading(true);
    setBrowserMsg(null);
    try {
      const res = await api.initBrowser();
      if (res.ok && res.data) {
        setBrowserMsg('Steel 远程浏览器已成功就绪！');
        setTimeout(() => setBrowserMsg(null), 4000);
      } else {
        alert(res.error?.message || '浏览器初始化失败');
      }
    } catch (e: any) {
      alert('请求失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  const handleLogin = async () => {
    setLoading(true);
    try {
      const res = await api.ensureAuth();
      if (res.ok) {
        await refreshAuth();
        alert('登录成功！已进入票根发票系统。');
      } else if (res.error?.code === 'HUMAN_ACTION_REQUIRED' && res.error.human_action) {
        window.open(res.error.human_action.url, '_blank');
        await refreshAuth();
      } else {
        alert('登录未完成: ' + (res.error?.message || '未知错误'));
      }
    } catch (e: any) {
      alert('登录请求失败: ' + e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshAuth();
  }, []);

  const isLoggedIn = auth?.status === 'LOGGED_IN';
  const username = auth?.session?.profile?.username || '13602004317';

  return (
    <AuthContext.Provider
      value={{
        auth,
        loading,
        refreshAuth,
        handleInitBrowser,
        handleLogin,
      }}
    >
      <div className="min-h-screen flex flex-col bg-slate-50 text-slate-900">
        <header className="bg-white border-b border-slate-200 sticky top-0 z-30 shadow-xs">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
            {/* Logo & Identity */}
            <div className="flex items-center gap-3">
              <div className="bg-blue-600 text-white font-bold px-2.5 py-1 rounded text-sm tracking-wider shadow-xs">
                TXFFP
              </div>
              <div>
                <span className="font-bold text-slate-900 tracking-tight text-base">票根自动开票 Agent</span>
                <span className="hidden sm:inline-block ml-2 text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">
                  v0.1.0
                </span>
              </div>
            </div>

            {/* Navigation Tabs */}
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
                {!isLoggedIn && <Lock className="w-3 h-3 text-slate-400" />}
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
                {!isLoggedIn && <Lock className="w-3 h-3 text-slate-400" />}
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

            {/* Header Action Buttons */}
            <div className="flex items-center gap-2.5">
              {/* Status Badge */}
              <div
                className={`hidden md:flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border ${
                  isLoggedIn
                    ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                    : 'bg-amber-50 text-amber-700 border-amber-200'
                }`}
              >
                <span className={`w-2 h-2 rounded-full ${isLoggedIn ? 'bg-emerald-500' : 'bg-amber-500'}`}></span>
                {isLoggedIn ? `已登录: ${username}` : '未登录'}
              </div>

              {/* Init Browser Button */}
              <button
                onClick={handleInitBrowser}
                disabled={loading}
                title="初始化并预热 Steel 远程浏览器"
                className="flex items-center gap-1.5 bg-white border border-slate-200 hover:bg-slate-50 text-slate-700 px-3 py-1.5 rounded-md text-xs font-medium transition shadow-2xs"
              >
                <Compass className="w-3.5 h-3.5 text-blue-600" />
                <span className="hidden sm:inline">初始化浏览器</span>
              </button>

              {/* Login Account Button */}
              <button
                onClick={handleLogin}
                disabled={loading}
                title={isLoggedIn ? '重新验证并同步会话' : '执行自动登录流程'}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition shadow-2xs ${
                  isLoggedIn
                    ? 'bg-slate-100 hover:bg-slate-200 text-slate-700'
                    : 'bg-blue-600 hover:bg-blue-700 text-white shadow-sm'
                }`}
              >
                <LogIn className="w-3.5 h-3.5" />
                <span>{isLoggedIn ? '重新登录' : '登录账户'}</span>
              </button>
            </div>
          </div>
        </header>

        {browserMsg && (
          <div className="bg-blue-50 border-b border-blue-200 text-blue-800 text-xs py-2 px-4 text-center font-medium">
            {browserMsg}
          </div>
        )}

        <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route
              path="/invoice"
              element={
                <AuthGuard>
                  <Invoice />
                </AuthGuard>
              }
            />
            <Route
              path="/invoices"
              element={
                <AuthGuard>
                  <Invoices />
                </AuthGuard>
              }
            />
            <Route path="/human-actions" element={<HumanActions />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>
    </AuthContext.Provider>
  );
};

export const App: React.FC = () => {
  return (
    <BrowserRouter>
      <AppContent />
    </BrowserRouter>
  );
};

