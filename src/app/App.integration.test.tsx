import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(async (command: string) => {
    if (["list_duty_plans", "list_common_routes", "list_personnel"].includes(command)) return [];
    if (command === "latest_personnel_import_log") return null;
    return undefined;
  }),
}));

function defaultInvoke(command: string) {
  if (["list_duty_plans", "list_common_routes", "list_personnel"].includes(command)) return Promise.resolve([]);
  if (command === "latest_personnel_import_log") return Promise.resolve(null);
  return Promise.resolve(undefined);
}

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../features/map/MapCanvas", () => ({ MapCanvas: () => <div>地圖測試替身</div> }));
vi.mock("../features/map/CustomBasemapCanvas", () => ({ CustomBasemapCanvas: () => <div>底圖測試替身</div> }));
vi.mock("../features/map/CustomBasemapOutput", () => ({ CustomBasemapOutput: () => <div>輸出測試替身</div> }));

import App from "./App";

describe("勤務建立流程", () => {
  beforeEach(() => { invoke.mockReset(); invoke.mockImplementation(defaultInvoke); });

  it("可從首頁切換至建立勤務表單，並改選地圖模式", async () => {
    render(<App />);
    expect(await screen.findByRole("button", { name: "新增勤務" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "新增勤務" }));
    expect(screen.getByLabelText("勤務計畫名稱")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("勤務計畫名稱"), { target: { value: "測試勤務" } });
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "map" } });
    expect(screen.queryByText("匯入勤務簡圖")).toBeNull();
    expect((screen.getByLabelText("勤務計畫名稱") as HTMLInputElement).value).toBe("測試勤務");
  });

  it("開啟既有工作區時只呼叫 Rust 原生選檔命令", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "開啟資料夾" }));
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("select_workspace_file"));
  });

  it("安全資料庫無法啟動時顯示可讀診斷，而非嘗試載入工作區", async () => {
    invoke.mockImplementation((command: string) => command === "startup_status"
      ? Promise.reject(new Error("金鑰已遺失"))
      : defaultInvoke(command));
    render(<App />);
    expect(await screen.findByText("無法開啟安全資料庫")).toBeTruthy();
    expect(screen.getByText(/金鑰已遺失/)).toBeTruthy();
    expect(invoke).not.toHaveBeenCalledWith("list_duty_plans");
  });
});
