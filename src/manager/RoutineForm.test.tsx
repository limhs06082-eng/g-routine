import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RoutineForm } from "./RoutineForm";

test("shows validation error and does not submit empty title", async () => {
  const onSubmit = vi.fn();
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(screen.getByRole("alert")).toHaveTextContent("루틴 이름을 입력해 주세요");
  expect(onSubmit).not.toHaveBeenCalled();
});

test("requires a weekday when repeat is weekdays", async () => {
  const onSubmit = vi.fn();
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), "주간학습안내");
  await user.click(screen.getByRole("radio", { name: "요일 지정" }));
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(screen.getByRole("alert")).toHaveTextContent("요일을 하나 이상 골라 주세요");
});

test("submits normalized input and resets", async () => {
  const onSubmit = vi.fn().mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), " 급식 지도 ");
  await user.click(screen.getByRole("radio", { name: "요일 지정" }));
  await user.click(screen.getByRole("button", { name: "월" }));
  await user.click(screen.getByRole("button", { name: "수" }));
  await user.type(screen.getByLabelText("바로가기"), "https://a.b");
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(onSubmit).toHaveBeenCalledWith({
    title: "급식 지도",
    repeatType: "weekdays",
    weekdays: 5,
    onceDate: null,
    dueTime: null,
    link: "https://a.b",
    slot: null,
  });
  expect(screen.getByLabelText("루틴 이름")).toHaveValue("");
});

test("edit mode shows existing values and a save button", () => {
  render(
    <RoutineForm
      initial={{
        id: 1,
        title: "출결 확인",
        repeatType: "daily",
        weekdays: 0,
        onceDate: null,
        dueTime: "09:00",
        link: null,
        slot: null,
        sortOrder: 0,
        createdAt: "",
        archivedAt: null,
      }}
      onSubmit={vi.fn()}
      onCancel={vi.fn()}
    />,
  );
  expect(screen.getByLabelText("루틴 이름")).toHaveValue("출결 확인");
  expect(screen.getByLabelText("마감 시각")).toHaveValue("09:00");
  expect(screen.getByRole("button", { name: "저장" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "취소" })).toBeInTheDocument();
});

test("pressing Enter twice while saving submits only once", async () => {
  const onSubmit = vi.fn(() => new Promise<void>(() => {}));
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), "급식 지도");
  await user.keyboard("{Enter}{Enter}");
  expect(onSubmit).toHaveBeenCalledTimes(1);
});

test("weekend days and a time slot can be chosen", async () => {
  const onSubmit = vi.fn().mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(<RoutineForm onSubmit={onSubmit} />);
  await user.type(screen.getByLabelText("루틴 이름"), "토요 방과후 지도");
  await user.click(screen.getByRole("radio", { name: "요일 지정" }));
  await user.click(screen.getByRole("button", { name: "토" }));
  await user.click(screen.getByRole("radio", { name: "방과 후" }));
  await user.click(screen.getByRole("button", { name: "추가" }));
  expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ repeatType: "weekdays", weekdays: 32, slot: "after" }));
});
