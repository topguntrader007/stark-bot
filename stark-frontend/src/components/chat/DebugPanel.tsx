import { useState, useEffect, useCallback, useRef } from 'react';
import { ChevronDown, ChevronRight, DollarSign, Cpu, Clock, Globe, Terminal, Wrench, Brain, CheckCircle, XCircle, Loader2, Zap } from 'lucide-react';
import clsx from 'clsx';
import { useGateway } from '@/hooks/useGateway';
import type { ExecutionEvent, X402PaymentEvent } from '@/types';

interface DebugTask {
  id: string;
  parentId?: string;
  name: string;
  description?: string;
  activeForm?: string;
  taskType?: string;
  status: 'pending' | 'in_progress' | 'completed' | 'error';
  startTime?: number;
  endTime?: number;
  duration?: number;
  toolsCount?: number;
  tokensUsed?: number;
  linesRead?: number;
  children: DebugTask[];
  // New fields for richer info
  toolName?: string;
  toolParams?: Record<string, unknown>;
  result?: string;
  success?: boolean;
}

interface DebugPanelProps {
  className?: string;
}

// Tool icons mapping
const getToolIcon = (toolName?: string) => {
  if (!toolName) return <Wrench className="w-3 h-3" />;
  const name = toolName.toLowerCase();
  if (name.includes('web') || name.includes('fetch')) return <Globe className="w-3 h-3" />;
  if (name.includes('exec') || name.includes('shell') || name.includes('bash')) return <Terminal className="w-3 h-3" />;
  if (name.includes('skill')) return <Zap className="w-3 h-3" />;
  if (name.includes('read') || name.includes('file') || name.includes('list')) return <Cpu className="w-3 h-3" />;
  return <Wrench className="w-3 h-3" />;
};

export default function DebugPanel({ className }: DebugPanelProps) {
  const [executions, setExecutions] = useState<Map<string, DebugTask>>(new Map());
  const [payments, setPayments] = useState<X402PaymentEvent[]>([]);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [activeTab, setActiveTab] = useState<'tasks' | 'payments'>('tasks');
  const [, forceUpdate] = useState(0);
  const { on, off } = useGateway();

  // Store tool execution data to match with tasks
  const toolDataRef = useRef<Map<string, { params: Record<string, unknown>; result?: string; success?: boolean; duration?: number }>>(new Map());

  // Force re-render every second to update elapsed times
  useEffect(() => {
    const interval = setInterval(() => {
      forceUpdate(n => n + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const updateExecution = useCallback((executionId: string, updater: (task: DebugTask) => DebugTask) => {
    setExecutions((prev) => {
      const newMap = new Map(prev);
      const execution = newMap.get(executionId);
      if (execution) {
        newMap.set(executionId, updater(execution));
      }
      return newMap;
    });
  }, []);

  const handleExecutionStarted = useCallback((data: unknown) => {
    const event = data as ExecutionEvent;
    const mode = (data as Record<string, unknown>).mode as string || 'execute';

    const newExecution: DebugTask = {
      id: event.execution_id,
      name: mode === 'plan' ? 'Planning' : 'Processing',
      description: mode === 'plan' ? 'Creating execution plan...' : 'Processing request...',
      taskType: 'execution',
      status: 'in_progress',
      startTime: Date.now(),
      children: [],
    };

    setExecutions((prev) => {
      const newMap = new Map(prev);
      newMap.set(event.execution_id, newExecution);
      return newMap;
    });
  }, []);

  const handleExecutionThinking = useCallback((data: unknown) => {
    const event = data as ExecutionEvent;
    updateExecution(event.execution_id, (execution) => ({
      ...execution,
      activeForm: event.active_form || (data as Record<string, unknown>).text as string || 'Thinking...',
    }));
  }, [updateExecution]);

  const handleTaskStarted = useCallback((data: unknown) => {
    const event = data as ExecutionEvent & {
      id?: string;
      type?: string;
      description?: string;
    };

    // Extract tool name from description if it's a tool task
    const desc = event.description || '';
    const toolMatch = desc.match(/Using tool: (\w+)/);
    const toolName = toolMatch ? toolMatch[1] : undefined;

    const newTask: DebugTask = {
      id: event.id || event.task_id || crypto.randomUUID(),
      parentId: event.parent_task_id || (data as Record<string, unknown>).parent_id as string,
      name: event.name || desc || 'Task',
      description: desc,
      taskType: event.type || (data as Record<string, unknown>).type as string,
      activeForm: event.active_form,
      status: 'in_progress',
      startTime: Date.now(),
      children: [],
      toolName,
    };

    updateExecution(event.execution_id, (execution) => {
      const addToParent = (tasks: DebugTask[]): DebugTask[] => {
        return tasks.map((task) => {
          if (task.id === newTask.parentId) {
            return { ...task, children: [...task.children, newTask] };
          }
          return { ...task, children: addToParent(task.children) };
        });
      };

      if (!newTask.parentId || newTask.parentId === execution.id) {
        return { ...execution, children: [...execution.children, newTask] };
      }
      return { ...execution, children: addToParent(execution.children) };
    });
  }, [updateExecution]);

  // Listen for tool.execution to capture parameters
  const handleToolExecution = useCallback((data: unknown) => {
    const event = data as { tool_name: string; parameters: Record<string, unknown> };
    // Store params keyed by tool name (we'll match by timing/order)
    toolDataRef.current.set(`pending_${event.tool_name}`, { params: event.parameters });
  }, []);

  // Listen for tool.result to capture success/duration
  const handleToolResult = useCallback((data: unknown) => {
    const event = data as { tool_name: string; success: boolean; duration_ms: number };
    const key = `pending_${event.tool_name}`;
    const existing = toolDataRef.current.get(key);
    if (existing) {
      existing.success = event.success;
      existing.duration = event.duration_ms;
    }
  }, []);

  const handleTaskUpdated = useCallback((data: unknown) => {
    const event = data as ExecutionEvent & { metrics?: { tool_uses?: number; tokens_used?: number; lines_read?: number; duration_ms?: number } };
    if (!event.task_id) return;

    const metrics = event.metrics || {};

    updateExecution(event.execution_id, (execution) => {
      const updateTask = (tasks: DebugTask[]): DebugTask[] => {
        return tasks.map((task) => {
          if (task.id === event.task_id) {
            return {
              ...task,
              toolsCount: metrics.tool_uses ?? event.tools_count ?? task.toolsCount,
              tokensUsed: metrics.tokens_used ?? event.tokens_used ?? task.tokensUsed,
              linesRead: metrics.lines_read ?? task.linesRead,
              activeForm: event.active_form ?? task.activeForm,
            };
          }
          return { ...task, children: updateTask(task.children) };
        });
      };

      if (execution.id === event.task_id) {
        return {
          ...execution,
          toolsCount: metrics.tool_uses ?? event.tools_count ?? execution.toolsCount,
          tokensUsed: metrics.tokens_used ?? event.tokens_used ?? execution.tokensUsed,
          linesRead: metrics.lines_read ?? execution.linesRead,
          activeForm: event.active_form ?? execution.activeForm,
        };
      }
      return { ...execution, children: updateTask(execution.children) };
    });
  }, [updateExecution]);

  const handleTaskCompleted = useCallback((data: unknown) => {
    const event = data as ExecutionEvent & {
      status?: string;
      metrics?: { tool_uses?: number; tokens_used?: number; lines_read?: number; duration_ms?: number }
    };
    if (!event.task_id) return;

    const metrics = event.metrics || {};
    const statusStr = event.status || 'completed';
    const isError = statusStr.toLowerCase().includes('error');

    updateExecution(event.execution_id, (execution) => {
      const completeTask = (tasks: DebugTask[]): DebugTask[] => {
        return tasks.map((task) => {
          if (task.id === event.task_id) {
            return {
              ...task,
              status: isError ? 'error' : 'completed',
              success: !isError,
              result: statusStr !== 'completed' ? statusStr : undefined,
              endTime: Date.now(),
              duration: metrics.duration_ms ?? event.duration_ms ?? (Date.now() - (task.startTime || Date.now())),
              toolsCount: metrics.tool_uses ?? task.toolsCount,
              tokensUsed: metrics.tokens_used ?? task.tokensUsed,
              linesRead: metrics.lines_read ?? task.linesRead,
            };
          }
          return { ...task, children: completeTask(task.children) };
        });
      };

      return { ...execution, children: completeTask(execution.children) };
    });
  }, [updateExecution]);

  const handleExecutionCompleted = useCallback((data: unknown) => {
    const event = data as ExecutionEvent & { metrics?: { tool_uses?: number; tokens_used?: number; duration_ms?: number } };
    const metrics = event.metrics || {};

    updateExecution(event.execution_id, (execution) => ({
      ...execution,
      status: 'completed',
      endTime: Date.now(),
      duration: metrics.duration_ms ?? event.duration_ms ?? (Date.now() - (execution.startTime || Date.now())),
      toolsCount: metrics.tool_uses ?? execution.toolsCount,
      tokensUsed: metrics.tokens_used ?? execution.tokensUsed,
    }));
  }, [updateExecution]);

  const handleX402Payment = useCallback((data: unknown) => {
    const event = data as X402PaymentEvent;
    setPayments((prev) => [...prev, event]);
  }, []);

  useEffect(() => {
    on('execution.started', handleExecutionStarted);
    on('execution.thinking', handleExecutionThinking);
    on('execution.task_started', handleTaskStarted);
    on('execution.task_updated', handleTaskUpdated);
    on('execution.task_completed', handleTaskCompleted);
    on('execution.completed', handleExecutionCompleted);
    on('tool.execution', handleToolExecution);
    on('tool.result', handleToolResult);
    on('x402.payment', handleX402Payment);

    return () => {
      off('execution.started', handleExecutionStarted);
      off('execution.thinking', handleExecutionThinking);
      off('execution.task_started', handleTaskStarted);
      off('execution.task_updated', handleTaskUpdated);
      off('execution.task_completed', handleTaskCompleted);
      off('execution.completed', handleExecutionCompleted);
      off('tool.execution', handleToolExecution);
      off('tool.result', handleToolResult);
      off('x402.payment', handleX402Payment);
    };
  }, [on, off, handleExecutionStarted, handleExecutionThinking, handleTaskStarted, handleTaskUpdated, handleTaskCompleted, handleExecutionCompleted, handleToolExecution, handleToolResult, handleX402Payment]);

  const toggleCollapse = (taskId: string) => {
    setCollapsed((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(taskId)) {
        newSet.delete(taskId);
      } else {
        newSet.add(taskId);
      }
      return newSet;
    });
  };

  const formatDuration = (ms?: number): string => {
    if (!ms) return '';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  };

  const formatElapsed = (startTime?: number): string => {
    if (!startTime) return '';
    const elapsed = Date.now() - startTime;
    return formatDuration(elapsed);
  };

  const formatTimestamp = (ts: string): string => {
    const date = new Date(ts);
    return date.toLocaleTimeString();
  };

  const formatTime = (timestamp?: number): string => {
    if (!timestamp) return '';
    return new Date(timestamp).toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  const totalPayments = payments.reduce((sum, p) => {
    const amount = parseFloat(p.amount_formatted || '0');
    return sum + amount;
  }, 0);

  const renderTask = (task: DebugTask, depth: number = 0): JSX.Element => {
    const hasChildren = task.children.length > 0;
    const isCollapsed = collapsed.has(task.id);
    const isInProgress = task.status === 'in_progress';

    // Status indicators
    const StatusIcon = () => {
      switch (task.status) {
        case 'pending':
          return <span className="text-slate-500">○</span>;
        case 'in_progress':
          return <Loader2 className="w-3.5 h-3.5 text-cyan-400 animate-spin" />;
        case 'completed':
          return <CheckCircle className="w-3.5 h-3.5 text-green-400" />;
        case 'error':
          return <XCircle className="w-3.5 h-3.5 text-red-400" />;
        default:
          return null;
      }
    };

    const typeConfig: Record<string, { color: string; bg: string; icon: JSX.Element }> = {
      tool: { color: 'text-purple-400', bg: 'bg-purple-500/20', icon: getToolIcon(task.toolName) },
      thinking: { color: 'text-yellow-400', bg: 'bg-yellow-500/20', icon: <Brain className="w-3 h-3" /> },
      agent: { color: 'text-blue-400', bg: 'bg-blue-500/20', icon: <Cpu className="w-3 h-3" /> },
      execution: { color: 'text-cyan-400', bg: 'bg-cyan-500/20', icon: <Zap className="w-3 h-3" /> },
      plan: { color: 'text-orange-400', bg: 'bg-orange-500/20', icon: <Brain className="w-3 h-3" /> },
    };

    const config = typeConfig[task.taskType || ''] || { color: 'text-slate-400', bg: 'bg-slate-500/20', icon: <Cpu className="w-3 h-3" /> };

    const taskText = isInProgress && task.activeForm
      ? task.activeForm
      : task.description || task.name;

    return (
      <div key={task.id} className={clsx('border-l-2', task.status === 'error' ? 'border-red-500/50' : 'border-slate-700')}>
        <div
          className={clsx(
            'py-2 px-3 hover:bg-slate-800/50 text-sm transition-colors',
            isInProgress && 'bg-slate-800/30'
          )}
        >
          {/* Header row */}
          <div className="flex items-center gap-2">
            {/* Collapse toggle */}
            {hasChildren ? (
              <button
                onClick={() => toggleCollapse(task.id)}
                className="p-0.5 hover:bg-slate-700 rounded shrink-0"
              >
                {isCollapsed ? (
                  <ChevronRight className="w-3 h-3 text-slate-500" />
                ) : (
                  <ChevronDown className="w-3 h-3 text-slate-500" />
                )}
              </button>
            ) : (
              <div className="w-4 shrink-0" />
            )}

            {/* Status icon */}
            <div className="shrink-0">
              <StatusIcon />
            </div>

            {/* Task type badge with icon */}
            {task.taskType && (
              <span className={clsx(
                'flex items-center gap-1 text-xs px-1.5 py-0.5 rounded shrink-0',
                config.color,
                config.bg
              )}>
                {config.icon}
                <span>{task.toolName || task.taskType}</span>
              </span>
            )}

            {/* Timestamp */}
            <span className="text-[10px] text-slate-600 shrink-0">
              {formatTime(task.startTime)}
            </span>

            {/* Metrics on the right */}
            <div className="flex items-center gap-3 text-xs text-slate-500 ml-auto shrink-0">
              {/* Duration or elapsed time */}
              {(task.duration || isInProgress) && (
                <span className={clsx(
                  'flex items-center gap-1',
                  isInProgress && 'text-cyan-400'
                )}>
                  <Clock className="w-3 h-3" />
                  {task.duration ? formatDuration(task.duration) : formatElapsed(task.startTime)}
                </span>
              )}

              {/* Tool count */}
              {task.toolsCount !== undefined && task.toolsCount > 0 && (
                <span className="flex items-center gap-1" title="Tool calls">
                  <Wrench className="w-3 h-3" />
                  {task.toolsCount}
                </span>
              )}

              {/* Lines read */}
              {task.linesRead !== undefined && task.linesRead > 0 && (
                <span className="text-slate-600" title="Lines read">
                  {task.linesRead} lines
                </span>
              )}

              {/* Tokens */}
              {task.tokensUsed !== undefined && task.tokensUsed > 0 && (
                <span title="Tokens used">
                  {task.tokensUsed >= 1000
                    ? `${(task.tokensUsed / 1000).toFixed(1)}k tok`
                    : `${task.tokensUsed} tok`}
                </span>
              )}
            </div>
          </div>

          {/* Task description */}
          <div
            className={clsx(
              'ml-6 mt-1 font-mono text-xs',
              task.status === 'completed' && 'text-slate-400',
              task.status === 'error' && 'text-red-400',
              isInProgress && 'text-cyan-300',
              task.status === 'pending' && 'text-slate-300'
            )}
          >
            {taskText}
          </div>

          {/* Error/result message if present */}
          {task.result && task.status === 'error' && (
            <div className="ml-6 mt-1 text-xs text-red-400/80 bg-red-500/10 px-2 py-1 rounded font-mono">
              {task.result}
            </div>
          )}

          {/* Children count indicator when collapsed */}
          {hasChildren && isCollapsed && (
            <div className="ml-6 mt-1 text-[10px] text-slate-600">
              {task.children.length} subtask{task.children.length !== 1 ? 's' : ''} hidden
            </div>
          )}
        </div>

        {/* Children */}
        {hasChildren && !isCollapsed && (
          <div className="ml-3">
            {task.children.map((child) => renderTask(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  // Calculate stats
  const allTasks = Array.from(executions.values());
  const completedCount = allTasks.filter(t => t.status === 'completed').length;
  const errorCount = allTasks.filter(t => t.status === 'error').length;
  const inProgressCount = allTasks.filter(t => t.status === 'in_progress').length;

  return (
    <div className={clsx(
      'bg-slate-900 border border-slate-700 rounded-lg overflow-hidden',
      className
    )}>
      {/* Tab headers with stats */}
      <div className="flex border-b border-slate-700 sticky top-0 bg-slate-900 z-10">
        <button
          onClick={() => setActiveTab('tasks')}
          className={clsx(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            activeTab === 'tasks'
              ? 'bg-slate-800 text-white border-b-2 border-cyan-500'
              : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
          )}
        >
          <Cpu className="w-4 h-4 inline mr-2" />
          Tasks
          {executions.size > 0 && (
            <span className="ml-2 text-xs">
              {inProgressCount > 0 && <span className="text-cyan-400">{inProgressCount} running</span>}
              {completedCount > 0 && <span className="text-green-400 ml-1">✓{completedCount}</span>}
              {errorCount > 0 && <span className="text-red-400 ml-1">✗{errorCount}</span>}
            </span>
          )}
        </button>
        <button
          onClick={() => setActiveTab('payments')}
          className={clsx(
            'flex-1 px-4 py-2 text-sm font-medium transition-colors',
            activeTab === 'payments'
              ? 'bg-slate-800 text-white border-b-2 border-green-500'
              : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
          )}
        >
          <DollarSign className="w-4 h-4 inline mr-2" />
          x402
          {payments.length > 0 && (
            <span className="ml-2 text-xs text-green-400">
              ${totalPayments.toFixed(4)}
            </span>
          )}
        </button>
      </div>

      {/* Tab content */}
      <div className="max-h-[400px] overflow-y-auto overflow-x-hidden">
        {activeTab === 'tasks' && (
          <div className="p-2">
            {executions.size === 0 ? (
              <div className="text-center text-slate-500 py-8">
                <Cpu className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p>No executions yet</p>
                <p className="text-xs mt-1">Send a message to see activity</p>
              </div>
            ) : (
              Array.from(executions.values()).reverse().map((execution) => (
                <div key={execution.id} className="mb-3 last:mb-0">
                  {renderTask(execution)}
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'payments' && (
          <div className="p-2">
            {payments.length > 0 && (
              <div className="mb-4 p-3 bg-gradient-to-r from-green-500/10 to-emerald-500/10 rounded-lg border border-green-500/20">
                <div className="text-xs text-slate-400 uppercase tracking-wide">Total Spent</div>
                <div className="text-2xl font-bold text-green-400">
                  ${totalPayments.toFixed(6)}
                  <span className="text-sm font-normal text-slate-500 ml-2">USDC</span>
                </div>
                <div className="text-xs text-slate-500 mt-1">
                  {payments.length} transaction{payments.length !== 1 ? 's' : ''}
                </div>
              </div>
            )}

            {payments.length === 0 ? (
              <div className="text-center text-slate-500 py-8">
                <DollarSign className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p>No x402 payments yet</p>
                <p className="text-xs mt-1">Payments appear when using pay-per-use AI</p>
              </div>
            ) : (
              <div className="space-y-2">
                {payments.slice().reverse().map((payment, idx) => (
                  <div
                    key={idx}
                    className="p-3 bg-slate-800 rounded-lg border border-slate-700 hover:border-slate-600 transition-colors"
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <DollarSign className="w-4 h-4 text-green-400" />
                        <span className="text-lg font-semibold text-green-400">
                          ${payment.amount_formatted}
                        </span>
                        <span className="text-xs px-1.5 py-0.5 bg-blue-500/20 text-blue-400 rounded">
                          {payment.asset}
                        </span>
                      </div>
                      <span className="text-xs text-slate-500">
                        {formatTimestamp(payment.timestamp)}
                      </span>
                    </div>

                    <div className="text-xs text-slate-400 space-y-1 font-mono" style={{ wordBreak: 'break-all' }}>
                      <div className="flex">
                        <span className="text-slate-500 w-16 shrink-0 font-sans">To:</span>
                        <span className="text-slate-400">{payment.pay_to}</span>
                      </div>
                      {payment.resource && (
                        <div className="flex">
                          <span className="text-slate-500 w-16 shrink-0 font-sans">Resource:</span>
                          <span className="text-slate-400">{payment.resource}</span>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
