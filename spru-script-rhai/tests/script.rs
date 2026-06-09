use spru::item::IdT;
use spru_util::cloned;
use tagset::tagset;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru_script::script(include = [Impl])]
struct S {
    #[get]
    #[set]
    a: i64,

    #[get]
    #[set]
    b: Option<IdT<S>>,
}

#[spru_script::script(partial = Impl)]
impl S {
    #[create]
    fn create(val: i64, dummy: i64) -> cloned::Create<S> {
        cloned::create(Self {
            a: val,
            b: None,
        })
    }

    #[function]
    fn f() -> i64 {
        9i64
    }

    // #[set]
    // fn b(&self, value: Option<IdT<S>>) -> (spru_util::cloned::Update<Self>, ) {
    //     let mut a = self.clone();
    //     a.b = value;
    //     (spru_util::cloned::update(a), )
    // }
}

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(derive(Debug))]
#[tagset(S)]
struct MyState;

#[tagset(impl spru::Action {
    type State = MyState;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(derive(Debug, Clone))]
#[tagset(include(cloned::Actions<S>))]
struct MyAction;

struct MyLexicon;
impl spru_script::Lexicon for MyLexicon {
    type Language = spru_script_rhai::Rhai<MyAction, Self>;

    fn register<Storage>(registration: &mut <Self::Language as spru_script_base::Language>::Registration<'_>)
    where
        Storage: spru::item::Storage<State = MyState>,
    {
        S!(<Storage, MyAction> registration => S);
    }
}

#[test]
fn script() {
    use spru_script::LanguageExec as _;
    let storage = spru_util::storage::Standalone::<MyState>::new();
    let rhai = spru_script_rhai::Rhai::<MyAction, MyLexicon>::default();
    let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);
    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&());
    let s = interactor
        .create(cloned::create(S { a: 3i64, b: None, }));
    let root = s.id();

    interactor.flush().unwrap();

    let mut interactor = test_interactor.interactor::<MyAction, _, ()>(&root);

    let script = r#"
    let b = type["S"].create(7, 5);
    context.root.b = b;
    // let a = S.f();
    // a
    context.root.b.a
    "#;

    let a: i64 = rhai.exec(&mut interactor, script, rhai::Dynamic::UNIT)
        .unwrap();

    assert_eq!(a, 7i64);
}

// #[test]
// fn chaining() {
//     let mut rhai = rhai::Engine::new();
//     #[derive(Clone)]
//     struct A(B);
//     #[derive(Clone)]
//     struct B(i64);
//     let a_b = rhai::FuncRegistration::new_getter("b").with_purity(true);
//     a_b.register_into_engine(&mut rhai, |a: &mut A| a.0.clone());
//     let a_b = rhai::FuncRegistration::new_setter("b").with_purity(true);
//     a_b.register_into_engine(&mut rhai, |a: &mut A, b: B| println!("set A.b"));
//     let b_c = rhai::FuncRegistration::new_getter("b").with_purity(true);
//     b_c.register_into_engine(&mut rhai, |b: &mut B| b.0);
//     let b_c = rhai::FuncRegistration::new_setter("c").with_purity(true);
//     b_c.register_into_engine(&mut rhai, |b: &mut B, c: i64| println!("set B.c = {c}"));
//     let b_c_method = rhai::FuncRegistration::new("set_c").with_purity(true);
//     b_c_method.register_into_engine(&mut rhai, |b: &mut B, c: i64| println!("set method B.c = {c}"));
//     rhai.register_fn("new_a", || A(B(0)));

//     let script = r#"
//         let a = new_a();
//         let b = a.b;
//         b.c = 5;
//         b.set_c(6);
//     "#;

//     let _: rhai::Dynamic = rhai.eval(script).unwrap();
// }