import React, { useEffect, useState } from 'react';
import { api, AuthState } from '../api/client';
import { ExternalLink, ShieldAlert } from 'lucide-react';

export const HumanActions: React.FC = () => {
  const [auth, setAuth] = useState<AuthState | null>(null);

  useEffect(() => {
    api.getAuthStatus().then((res) => {
      if (res.ok && res.data) setAuth(res.data);
    });
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">人工处理通道 (Human Actions)</h1>
        <p className="text-slate-500 text-sm mt-1">处理票根网出现的滑块验证、短信验证码或未知页面接管</p>
      </div>

      {auth?.active_human_action_id ? (
        <div className="bg-amber-50 border border-amber-200 rounded-lg p-5">
          <div className="flex items-center gap-2 text-amber-900 font-bold">
            <ShieldAlert className="w-5 h-5 text-amber-600" />
            有正在等待用户处理的验证任务
          </div>
          <p className="text-amber-800 text-sm mt-2">{auth.message}</p>
          <div className="mt-4">
            <a
              href={`/human/${auth.active_human_action_id}`}
              target="_blank"
              rel="noreferrer"
              className="bg-amber-600 hover:bg-amber-700 text-white px-4 py-2 rounded text-sm font-medium inline-flex items-center gap-2"
            >
              <ExternalLink className="w-4 h-4" />
              打开专属处理页面
            </a>
          </div>
        </div>
      ) : (
        <div className="bg-white border border-slate-200 rounded-lg p-8 text-center text-slate-500">
          目前没有待处理的人工验证任务。所有自动化流程均在稳定运行中。
        </div>
      )}
    </div>
  );
};
