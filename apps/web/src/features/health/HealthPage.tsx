import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { HeartPulse, Moon, Activity } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Card, CardContent, EmptyState, MetricCard, PageHeader, Section } from "../../components/ui";
import { entities, number, recentDays, text } from "../../lib/entities";

export function HealthPage(){
  const {state}=useApp(); const workouts=entities(state,"workout.workout"); const days=recentDays(14); const data=days.map((day)=>({day:day.slice(5),minutes:Math.round(workouts.filter((w)=>text(w,"localDate")===day).reduce((s,w)=>s+number(w,"durationSeconds"),0)/60)})); const activeDays=data.filter((item)=>item.minutes>0).length; const totalMinutes=data.reduce((s,item)=>s+item.minutes,0);
  return <div className="page-shell"><PageHeader title="健康"/><div className="grid gap-3 sm:grid-cols-3"><MetricCard label="14 天活跃天数" value={`${activeDays} 天`} icon={<Activity size={17}/>}/><MetricCard label="14 天训练时长" value={`${totalMinutes} 分钟`} icon={<HeartPulse size={17}/>}/><MetricCard label="睡眠/恢复" value="待接入" icon={<Moon size={17}/>} /></div><Section className="mt-6" title="活动趋势"><Card><CardContent className="pt-5"><div className="h-64"><ResponsiveContainer width="100%" height="100%"><AreaChart data={data}><XAxis dataKey="day" tickLine={false} axisLine={false} tick={{fontSize:10}}/><Tooltip/><Area type="monotone" dataKey="minutes" stroke="hsl(var(--chart-2))" fill="hsl(var(--chart-2))" fillOpacity={0.12} strokeWidth={2}/></AreaChart></ResponsiveContainer></div></CardContent></Card></Section><div className="mt-5"><EmptyState title="更多健康数据待接入" /></div></div>;
}
