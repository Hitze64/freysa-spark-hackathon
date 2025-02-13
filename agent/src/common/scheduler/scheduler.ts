import cron, { ScheduledTask } from "node-cron"

type ScheduleHandle = {
  stop: () => void
}

export class Scheduler {
  private timezone: string

  /**
   * Initializes the Scheduler with a specific timezone.
   * @param timezone The timezone to use for scheduling tasks.
   */
  constructor(timezone: string = "Etc/UTC") {
    this.timezone = timezone
  }

  /**
   * Schedules a job that runs on a CRON string expression.
   * @param cronExpression A valid 6-field CRON expression for node-cron.
   * @param task The function to execute on schedule.
   * @returns A handle that can stop the scheduled task.
   */
  public schedule(cronExpression: string, task: () => void): ScheduleHandle {
    const scheduledTask: ScheduledTask = cron.schedule(cronExpression, task, {
      scheduled: true,
      timezone: this.timezone, // Use the specified timezone
    })
    return {
      stop: () => scheduledTask.stop(),
    }
  }

  /**
   * Schedules a job every N seconds.
   */
  public scheduleEverySeconds(
    interval: number,
    task: () => void
  ): ScheduleHandle {
    // node-cron uses the first field as "seconds"
    // Format: "*/N * * * * *" means "every N seconds"
    const cronExpression = `*/${interval} * * * * *`
    return this.schedule(cronExpression, task)
  }

  /**
   * Schedules a job every N minutes.
   */
  public scheduleEveryMinutes(
    interval: number,
    task: () => void
  ): ScheduleHandle {
    // We fix the seconds to 0, run every N minutes:
    // Format: "0 */N * * * *" means "at second 0 of every N-th minute"
    const cronExpression = `0 */${interval} * * * *`
    return this.schedule(cronExpression, task)
  }

  /**
   * Schedules a job every N hours.
   */
  public scheduleEveryHours(
    interval: number,
    task: () => void
  ): ScheduleHandle {
    // We fix the seconds and minutes to 0, run every N hours:
    // Format: "0 0 */N * * *" means "at second 0, minute 0 of every N-th hour"
    const cronExpression = `0 0 */${interval} * * *`
    return this.schedule(cronExpression, task)
  }

  /**
   * Schedules a job at a particular time each day (e.g., 3:30 PM).
   * @param hour 0-23
   * @param minute 0-59
   * @param second 0-59 (optional, default 0)
   */
  public scheduleDaily(
    hour: number,
    minute: number,
    second = 0,
    task: () => void
  ): ScheduleHandle {
    // Format: "second minute hour * * *" runs every day at that time
    const cronExpression = `${second} ${minute} ${hour} * * *`
    return this.schedule(cronExpression, task)
  }

  /**
   * Schedules a job at a particular time on specific days of the week.
   * @param hour 0-23
   * @param minute 0-59
   * @param daysOfWeek array of days (0=Sunday, 1=Monday, etc.)
   * @param second 0-59 (optional, default 0)
   */
  public scheduleWeekly(
    hour: number,
    minute: number,
    daysOfWeek: number[],
    task: () => void,
    second = 0
  ): ScheduleHandle {
    // CRON day-of-week field is the 6th field, e.g. "1,3,5" for Mon,Wed,Fri
    const dayOfWeekExpression = daysOfWeek.join(",")
    const cronExpression = `${second} ${minute} ${hour} * * ${dayOfWeekExpression}`
    return this.schedule(cronExpression, task)
  }

  /**
   * Schedules a job at a particular time on specific days of the month.
   * @param hour 0-23
   * @param minute 0-59
   * @param daysOfMonth array of days (1-31)
   * @param second 0-59 (optional, default 0)
   */
  public scheduleMonthly(
    hour: number,
    minute: number,
    daysOfMonth: number[],
    task: () => void,
    second = 0
  ): ScheduleHandle {
    // CRON day-of-month field is the 4th field, e.g. "1,15,31"
    const dayOfMonthExpression = daysOfMonth.join(",")
    const cronExpression = `${second} ${minute} ${hour} ${dayOfMonthExpression} * *`
    return this.schedule(cronExpression, task)
  }
}
