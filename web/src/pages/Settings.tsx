import React, { useEffect, useState } from 'react';
import { api, SettingsData } from '../api/client';
import { KeyRound, Server, Sliders, CheckCircle2 } from 'lucide-react';

export const Settings: React.FC = () => {
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [phone, setPhone] = useState('');
  const [saved, setSaved] = useState(false);

  const loadSettings = async () => {
    try {
      const res = await api.getSettings();
      if (res.ok && res.data) setSettings(res.data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    loadSettings();
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username || !password) {
      alert('请输入用户名和密码');
      return;
    }
    try {
      const res = await api.updateCredentials({ username, password, phone });
      if (res.ok) {
        setSaved(true);
        setPassword('');
        loadSettings();
        setTimeout(() => setSaved(false), 3000);
      } else {
        alert(res.error?.message || '保存失败');
      }
    } catch (err: any) {
      alert('请求错误: ' + err.message);
    }
  };

  const handleDelete = async () => {
    if (!window.confirm('确认清除已保存的票根凭据？')) return;
    try {
      const res = await api.deleteCredentials();
      if (res.ok) {
        setUsername('');
        setPassword('');
        setPhone('');
        loadSettings();
      }
    } catch (err: any) {
      alert('删除失败: ' + err.message);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">系统设置</h1>
        <p className="text-slate-500 text-sm mt-1">管理票根凭据（明文密码安全保护）、自动化策略与外部服务配置</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
          <div className="flex items-center gap-2 font-bold text-slate-800 border-b border-slate-100 pb-3">
            <KeyRound className="w-5 h-5 text-blue-600" />
            票根网凭据配置
          </div>

          <div className="bg-slate-50 rounded p-3 text-xs text-slate-600 space-y-1">
            <div>
              用户名状态:{' '}
              <span className="font-semibold text-slate-900">
                {settings?.credentials.username_configured ? '已配置' : '未配置'}
              </span>
            </div>
            <div>
              密码状态:{' '}
              <span className="font-semibold text-slate-900">
                {settings?.credentials.password_configured ? '已安全存储（不回显）' : '未配置'}
              </span>
            </div>
            <div>
              凭据存储来源:{' '}
              <span className="font-mono text-slate-900">{settings?.credentials.source || 'none'}</span>
            </div>
          </div>

          <form onSubmit={handleSave} className="space-y-3">
            <div>
              <label className="block text-xs font-semibold text-slate-600 uppercase mb-1">票根手机号 / 账号</label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="例如 13800000000"
                className="w-full border border-slate-200 rounded px-3 py-1.5 text-sm"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-600 uppercase mb-1">票根密码</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="输入新密码以更新"
                className="w-full border border-slate-200 rounded px-3 py-1.5 text-sm"
              />
            </div>

            <div className="pt-2 flex items-center justify-between">
              <button
                type="submit"
                className="bg-blue-600 hover:bg-blue-700 text-white font-medium py-1.5 px-4 rounded text-sm transition"
              >
                保存新凭据
              </button>
              {settings?.credentials.username_configured && (
                <button
                  type="button"
                  onClick={handleDelete}
                  className="text-rose-600 hover:text-rose-700 text-xs font-medium"
                >
                  清除已存凭据
                </button>
              )}
            </div>
          </form>

          {saved && (
            <div className="bg-emerald-50 text-emerald-800 text-xs p-2.5 rounded flex items-center gap-1.5">
              <CheckCircle2 className="w-4 h-4 text-emerald-600" />
              凭据已安全保存至后端 Secret 文件。
            </div>
          )}
        </div>

        <div className="space-y-6">
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
            <div className="flex items-center gap-2 font-bold text-slate-800 border-b border-slate-100 pb-3">
              <Sliders className="w-5 h-5 text-indigo-600" />
              自动化策略
            </div>
            <div className="space-y-2 text-sm text-slate-600">
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span>登录认证模式:</span>
                <span className="font-mono font-semibold text-slate-800 uppercase">{settings?.auth_mode}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span>自动登录执行:</span>
                <span className="font-semibold text-slate-800">{settings?.auto_login ? '已开启' : '关闭'}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-50">
                <span>最大安全重试次数:</span>
                <span className="font-mono font-semibold text-slate-800">{settings?.max_auth_attempts} 次</span>
              </div>
              <div className="flex justify-between py-1">
                <span>脱敏研究模式 (Research):</span>
                <span className="font-semibold text-slate-800">{settings?.research_mode ? '开启' : '关闭'}</span>
              </div>
            </div>
          </div>

          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-sm space-y-4">
            <div className="flex items-center gap-2 font-bold text-slate-800 border-b border-slate-100 pb-3">
              <Server className="w-5 h-5 text-emerald-600" />
              外部服务配置
            </div>
            <div className="space-y-2 text-xs text-slate-600">
              <div>
                <span className="text-slate-400 block mb-0.5">Steel 浏览器服务地址:</span>
                <span className="font-mono font-medium text-slate-800 bg-slate-50 p-1.5 rounded block">
                  {settings?.steel_base_url}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
