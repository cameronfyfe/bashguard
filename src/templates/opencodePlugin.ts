import type { Plugin } from "@opencode-ai/plugin"
import { execSync } from "child_process"

export const BashguardPlugin: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      // Only intercept bash tool calls
      if (input.tool !== "bash") return

      const command = output.args?.command
      if (!command) return

      try {
        // Call bashguard check with OpenCode format
        const result = execSync("bashguard check --json --format opencode", {
          input: JSON.stringify({
            session_id: input.sessionID || "opencode-session",
            tool_input: { command }
          }),
          encoding: "utf-8",
          timeout: 5000
        })

        const decision = JSON.parse(result)

        if (decision.abort) {
          throw new Error(`[bashguard] ${decision.abort}`)
        }
      } catch (error: any) {
        // Re-throw bashguard denials
        if (error?.message?.startsWith("[bashguard]")) {
          throw error
        }
        // On other errors, log and allow (fail-open for usability)
        console.error("[bashguard] Error:", error)
      }
    }
  }
}

export default BashguardPlugin
