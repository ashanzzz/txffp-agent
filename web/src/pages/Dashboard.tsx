import React, { useEffect, useState } from 'react';
import { api, AuthState } from '../api/client';
import { ShieldCheck, AlertCircle, CheckCircle, RefreshCw, ExternalLink, Activity, Sparkles } from 'lucide-react';

export const Dashboard: React.FC = () => {
  const [auth, setAuth] = useState<AuthState | null>(null);
  const [health, setHealth] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkMsg, setCheckMsg] = useState<string | null>(null);

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

  const handleCheckLiveness = async () => {
    setChecking(true);
    setCheckMsg(null);
    try {
      const res = await fetch('/api/v1/auth/check', { method: 'POST' });
      const data = await res.json();
      if (data.ok) {
        setCheckMsg(data.data.status === 'LOGGED_IN' ? '✅ 会话健康在线，已完成实时核验证明！' : '会话已自动恢复同步。');
        await loadData();
      } else {
        setCheckMsg(`⚠️ 会话状态: ${data.error?.message || '需重新登录'}`);
        await loadData();
      }
    } catch (e: any) {
      setCheckMsg('核验请求失败: ' + e.message);
    } finally {
      setChecking(false);
      setTimeout(() => setCheckMsg(null), 5000);
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

  const activeTab = health?.active_tab;

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row justify-between sm:items-center gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">系统仪表盘</h1>
          <p className="text-slate-500 text-sm mt-1">
            按需启动、毫秒级掉线检测与会话自愈监控
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleCheckLiveness}
            disabled={checking}
            className="flex items-center gap-1.5 bg-blue-50 border border-blue-200 px-3 py-1.5 rounded-md text-xs font-medium text-blue-700 hover:bg-blue-100 transition shadow-2xs"
          >
            <Activity className={`w-3.5 h-3.5 ${checking ? 'animate-pulse text-blue-600' : ''}`} />
            {checking ? '正在检测会话...' : '实时掉线检测与自愈'}
          </button>
          <button
            onClick={loadData}
            disabled={loading}
            className="flex items-center gap-1.5 bg-white border border-slate-200 px-3 py-1.5 rounded-md text-xs font-medium hover:bg-slate-50 text-slate-700 shadow-2xs"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            刷新状态
          </button>
        </div>
      </div>

      {checkMsg && (
        <div className="bg-emerald-50 border border-emerald-200 text-emerald-800 text-xs py-2.5 px-4 rounded-md flex items-center gap-2 animate-fadeIn">
          <Sparkles className="w-4 h-4 text-emerald-600 flex-shrink-0" />
          <span>{checkMsg}</span>
        </div>
      )}

      {/* Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-xs">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">后端核心服务</div>
          <div className="mt-3 flex items-center gap-2">
            <CheckCircle className="w-5 h-5 text-emerald-600" />
            <span className="text-lg font-bold text-slate-900">{health?.service || 'txffp-server'}</span>
            <span className="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">
              v{health?.version || '0.1.0'}
            </span>
          </div>
          <div className="mt-2 text-xs text-slate-500">SQLite: {health?.database || '已就绪'} (WAL 模式)</div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-xs">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">票根网认证状态</div>
          <div className="mt-3 flex items-center gap-2">
            {auth?.status === 'LOGGED_IN' ? (
              <ShieldCheck className="w-5 h-5 text-emerald-600" />
            ) : (
              <AlertCircle className="w-5 h-5 text-amber-500" />
            )}
            <span className="text-lg font-bold text-slate-900">{auth?.status || 'UNKNOWN'}</span>
          </div>
          <div className="mt-2 text-xs text-slate-500 truncate" title={auth?.message}>
            {auth?.message || '检查中...'}
          </div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-xs">
          <div className="text-xs font-semibold text-slate-400 uppercase tracking-wider">Steel 浏览器连通性</div>
          <div className="mt-3 flex items-center gap-2">
            {health?.steel_connected ? (
              <CheckCircle className="w-5 h-5 text-emerald-600" />
            ) : (
              <AlertCircle className="w-5 h-5 text-rose-500" />
            )}
            <span className="text-lg font-bold text-slate-900">{health?.steel_connected ? '正常连接' : '未连接'}</span>
          </div>
          <div className="mt-2 text-xs text-slate-500">按需启动 · 无任务时自动休眠</div>
        </div>
      </div>

      {/* Active Tab Monitor Card */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-xs space-y-3">
        <div className="flex items-center justify-between border-b border-slate-100 pb-3">
          <div className="flex items-center gap-2">
            <span className="w-2.5 h-2.5 rounded-full bg-blue-600"></span>
            <h2 className="text-sm font-bold text-slate-800">当前活跃标签页监控 (Active Tab)</h2>
            <span className="text-xs text-slate-400">（仅展示票根网作用域，隔离其他容器）</span>
          </div>
          {activeTab?.viewer_url && (
            <a
              href={activeTab.viewer_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-1.5 text-xs text-blue-600 hover:text-blue-700 bg-blue-50 hover:bg-blue-100 px-2.5 py-1 rounded font-medium transition"
            >
              <ExternalLink className="w-3.5 h-3.5" />
              打开远程实时视窗 (Screencast)
            </a>
          )}
        </div>

        {activeTab ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs">
            <div>
              <span className="text-slate-400 block mb-0.5">页面标题:</span>
              <span className="font-semibold text-slate-800 block truncate" title={activeTab.title}>
                {activeTab.title || '票根网'}
              </span>
            </div>
            <div>
              <span className="text-slate-400 block mb-0.5">当前网址:</span>
              <span className="font-mono text-slate-700 block truncate bg-slate-50 p-1.5 rounded" title={activeTab.url}>
                {activeTab.url}
              </span>
            </div>
          </div>
        ) : (
          <div className="text-xs text-slate-500 py-2">
            目前无驻留标签页（处于轻量低资源休眠状态；当发起登录、查询或开票时将自动按需唤醒）。
          </div>
        )}
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
