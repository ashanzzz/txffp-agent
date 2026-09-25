import React, { useEffect, useState } from 'react';
import { api, AuthState } from '../api/client';
import { ShieldCheck, AlertCircle, CheckCircle, RefreshCw } from 'lucide-react';

export const Dashboard: React.FC = () => {
  const [auth, setAuth] = useState<AuthState | null>(null);
  const [health, setHealth] = useState<any>(null);
  const [loading, setLoading] = useState(false);

  const loadData = async () => {
    setLoading(true);
    try {
      const [h, a] = await Promise.all([api.getHealth(), api.getAuthStatus()]);
      if (h.ok) setHealth(h.data);
      if (a.ok && a.data) setAuth(a.data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleEnsureAuth = async () => {
    try {
      const res = await api.ensureAuth();
      if (!res.ok && res.error?.code === 'HUMAN_ACTION_REQUIRED' && res.error.human_action) {
        window.open(res.error.human_action.url, '_blank');
      }
      loadData();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">系统仪表盘</h1>
          <p className="text-slate-500 text-sm mt-1">监控服务健康状态、票根网会话有效性与人工操作流转</p>
        </div>
        <button
          onClick={loadData}
          disabled={loading}
          className="flex items-center gap-2 bg-white border border-slate-200 px-3 py-1.5 rounded-md text-sm font-medium hover:bg-slate-50 text-slate-700 shadow-sm"
        >
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          刷新状态
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">后端核心服务</div>
          <div className="mt-3 flex items-center gap-2">
            <CheckCircle className="w-5 h-5 text-emerald-600" />
            <span className="text-lg font-bold">{health?.service || 'txffp-server'}</span>
            <span className="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">v{health?.version || '0.1.0'}</span>
          </div>
          <div className="mt-2 text-xs text-slate-500">SQLite: {health?.database || '已就绪'}</div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">票根网认证状态</div>
          <div className="mt-3 flex items-center gap-2">
            {auth?.status === 'LOGGED_IN' ? (
              <ShieldCheck className="w-5 h-5 text-emerald-600" />
            ) : (
              <AlertCircle className="w-5 h-5 text-amber-500" />
            )}
            <span className="text-lg font-bold">{auth?.status || 'UNKNOWN'}</span>
          </div>
          <div className="mt-2 text-xs text-slate-500">{auth?.message || '检查中...'}</div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">Steel 浏览器连通性</div>
          <div className="mt-3 flex items-center gap-2">
            {health?.steel_connected ? (
              <CheckCircle className="w-5 h-5 text-emerald-600" />
            ) : (
              <AlertCircle className="w-5 h-5 text-rose-500" />
            )}
            <span className="text-lg font-bold">{health?.steel_connected ? '正常连接' : '未连接'}</span>
          </div>
          <div className="mt-2 text-xs text-slate-500">宿主机无头浏览器沙箱服务</div>
        </div>
      </div>

      {auth?.status !== 'LOGGED_IN' && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 flex items-center justify-between">
          <div>
            <div className="font-semibold text-blue-900 text-sm">票根会话尚未登录或已失效</div>
            <div className="text-blue-700 text-xs mt-0.5">点击下方按钮可触发自动登录流程或弹出人机验证窗口。</div>
          </div>
          <button
            onClick={handleEnsureAuth}
            className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md text-sm font-medium shadow-sm transition"
          >
            发起登录验证
          </button>
        </div>
      )}
    </div>
  );
};
